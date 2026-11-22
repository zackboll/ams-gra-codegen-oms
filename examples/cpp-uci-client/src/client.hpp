// Receiver core of the standalone C++ UCI client.
//
// Client composes the EXISTING C++ LA-CAL runtime (ams_gra::lacal::Runtime)
// with the production-GENERATED receive endpoint and codec. It adds no
// transport, codec or protocol logic of its own:
//
//   connect()    Runtime::connect: TCP, WebSocket upgrade offering `owp`
//                (response must select exactly `owp`), verbose INIT, +OK, INFO.
//   subscribe()  Generated receive endpoints: one SUB per call, +OK / -ERR.
//   next()       one typed summary (FIFO), one runtime event, or idle.
//   shutdown()   UNSUB in reverse order, close the queue, Runtime::close() (joins
//                the runtime's threads); idempotent; main thread only.
//
// Every method is called from ONE application thread. The typed callback runs
// on the runtime's dispatcher thread and touches only `Receiver` (atomics and
// the bounded queue). No application lock is held while a runtime operation
// runs, and the runtime is never closed or destroyed from its handler.
#pragma once

#include <array>
#include <atomic>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <variant>
#include <vector>

#include "ams_gra/lacal/runtime.hpp"
#include "bounded_queue.hpp"
#include "cli.hpp"
#include "log_summary.hpp"
#include "oam.hpp"
#include "process_resources.hpp"
#include "release.hpp"
#include "service_api.hpp"
#include "service_codec.hpp"
#include "session.hpp"

// Bind the generated payload type to the generated codec. This receiver only
// decodes; the runtime's binding contract names both generated functions.
AMS_GRA_LACAL_PAYLOAD_CODEC(::programs::oam::LogMT, ::service_codec::encode_LogMT, ::service_codec::decode_LogMT)
#if CPP_UCI_CLIENT_WITH_RDMA
#include "rdma_summary.hpp"
AMS_GRA_LACAL_PAYLOAD_CODEC(::programs::oam::RDMAInitializeSetupMT, ::service_codec::encode_RDMAInitializeSetupMT,
                            ::service_codec::decode_RDMAInitializeSetupMT)
#endif
#if CPP_UCI_CLIENT_WITH_SURVIVABILITY
#include "survivability_summary.hpp"
AMS_GRA_LACAL_PAYLOAD_CODEC(::programs::oam::SurvivabilityRiskLevelMT, ::service_codec::encode_SurvivabilityRiskLevelMT,
                            ::service_codec::decode_SurvivabilityRiskLevelMT)
#endif

namespace cpp_uci_client {

enum class Family : std::size_t { log, rdma, survivability };
constexpr std::size_t index(Family family) noexcept { return static_cast<std::size_t>(family); }
#if CPP_UCI_CLIENT_WITH_SURVIVABILITY
inline constexpr std::array<Family, 3> families{Family::log, Family::rdma, Family::survivability};
#else
inline constexpr std::array<Family, 2> families{Family::log, Family::rdma};
#endif
inline constexpr std::string_view message_name(Family family) noexcept {
    switch (family) {
        case Family::log: return release::global_message;
        case Family::rdma: return release::rdma_message;
        case Family::survivability: return release::survivability_message;
    }
    return {};
}

/// Fixed, bounded runtime settings (only url/service_id come from the CLI).
struct ClientSettings {
    std::string url;
    std::string service_id;
    std::size_t queue_capacity = 64;                     // application hand-off bound
    std::chrono::milliseconds connect_timeout{5000};     // TCP + upgrade + INIT/+OK/INFO
    std::chrono::milliseconds operation_timeout{5000};   // SUB / UNSUB acknowledgment
    std::chrono::milliseconds close_timeout{2000};       // WebSocket close handshake
    std::size_t max_message_bytes = 1024U * 1024U;       // one incoming frame
    std::size_t max_pending_deliveries = 1024;           // runtime MSG backlog (count)
    std::size_t max_pending_delivery_bytes = 16U * 1024U * 1024U;
    std::size_t max_pending_events = 256;
    std::size_t max_pending_event_bytes = 256U * 1024U;
};

using Extractor = std::function<LogSummary(const ::programs::oam::LogMT&)>;
#if CPP_UCI_CLIENT_WITH_RDMA
using RdmaExtractor = std::function<RdmaSummary(const ::programs::oam::RDMAInitializeSetupMT&)>;
#endif
#if CPP_UCI_CLIENT_WITH_SURVIVABILITY
using SurvivabilityExtractor = std::function<SurvivabilitySummary(const ::programs::oam::SurvivabilityRiskLevelMT&)>;
using Summary = std::variant<LogSummary, RdmaSummary, SurvivabilitySummary>;
#elif CPP_UCI_CLIENT_WITH_RDMA
using Summary = std::variant<LogSummary, RdmaSummary>;
#else
using Summary = LogSummary;
#endif

struct Delivered {
    std::uint64_t sequence = 0;  // display order, not cross-subscription publication order
    Summary summary;
};

inline std::string format_delivered(const Delivered& item) {
    const auto format = [&item](const auto& summary) {
        return format_summary(item.sequence, summary, release::schema_version);
    };
#if CPP_UCI_CLIENT_WITH_RDMA
    return std::visit(format, item.summary);
#else
    return format(item.summary);
#endif
}

/// The only state the runtime's dispatcher thread touches.
class Receiver {
public:
    Receiver(std::size_t capacity, Extractor extract
#if CPP_UCI_CLIENT_WITH_RDMA
             , RdmaExtractor extract_rdma
#endif
#if CPP_UCI_CLIENT_WITH_SURVIVABILITY
             , SurvivabilityExtractor extract_survivability
#endif
             ) : queue_(capacity), extract_(std::move(extract))
#if CPP_UCI_CLIENT_WITH_RDMA
                 , extract_rdma_(std::move(extract_rdma))
#endif
#if CPP_UCI_CLIENT_WITH_SURVIVABILITY
                 , extract_survivability_(std::move(extract_survivability))
#endif
    {}

    /// The typed callback body: copy the selected fields, hand off, return.
    void on_log(const ::programs::oam::LogMT& log) {
        on_entry(Family::log);
        (void)queue_.push(extract_(log));  // never waits; full => sticky overflow
    }
#if CPP_UCI_CLIENT_WITH_RDMA
    void on_rdma(const ::programs::oam::RDMAInitializeSetupMT& setup) {
        on_entry(Family::rdma);
        (void)queue_.push(extract_rdma_(setup));
    }
#endif
#if CPP_UCI_CLIENT_WITH_SURVIVABILITY
    void on_survivability(const ::programs::oam::SurvivabilityRiskLevelMT& risk) {
        on_entry(Family::survivability);
        (void)queue_.push(extract_survivability_(risk));
    }
#endif

    BoundedQueue<Summary>& queue() noexcept { return queue_; }
    void mark_unsubscribed(Family family) noexcept { counters_[index(family)].unsubscribed.store(true); }
    std::uint64_t callbacks(Family family) const noexcept {
        return index(family) < counters_.size() ? counters_[index(family)].callbacks.load() : 0;
    }
    std::uint64_t callbacks() const noexcept {
        std::uint64_t count = 0;
        for (Family family : families) {
            count += callbacks(family);
        }
        return count;
    }
    std::uint64_t callbacks_after_unsubscribe(Family family) const noexcept {
        return index(family) < counters_.size() ? counters_[index(family)].callbacks_after_unsubscribe.load() : 0;
    }
    std::uint64_t callbacks_after_unsubscribe() const noexcept {
        std::uint64_t count = 0;
        for (Family family : families) {
            count += callbacks_after_unsubscribe(family);
        }
        return count;
    }

private:
    struct CallbackCounters {
        std::atomic<std::uint64_t> callbacks{0};
        std::atomic<std::uint64_t> callbacks_after_unsubscribe{0};
        std::atomic<bool> unsubscribed{false};
    };
    void on_entry(Family family) noexcept {
        auto& counters = counters_[index(family)];
        ++counters.callbacks;
        if (counters.unsubscribed.load()) {
            ++counters.callbacks_after_unsubscribe;
        }
    }

    BoundedQueue<Summary> queue_;
    Extractor extract_;
#if CPP_UCI_CLIENT_WITH_RDMA
    RdmaExtractor extract_rdma_;
#endif
#if CPP_UCI_CLIENT_WITH_SURVIVABILITY
    SurvivabilityExtractor extract_survivability_;
#endif
    std::array<CallbackCounters, families.size()> counters_{};
};

struct Failure {
    int exit_status = exit_status::internal;
    std::string line;
};

struct Next {
    enum class Kind { message, notice, idle, failure };
    Kind kind = Kind::idle;
    Delivered item;       // Kind::message
    std::string line;     // Kind::notice / Kind::failure
    int exit_status = 0;  // Kind::failure
};

struct ShutdownReport {
    bool subscribed = false;
    bool unsubscribe_ok = true;
    std::vector<std::string> unsubscribe_lines;  // reverse SUB order
    bool connected_runtime = false;
    bool close_ok = true;
    bool session_failed = false;      // a late failure observed during combined-mode teardown
    std::string close_line;
    std::vector<std::string> notices;  // events drained during shutdown
    std::size_t pending_discarded = 0; // delivered to the queue, never displayed
    std::uint64_t callbacks = 0;
    std::uint64_t callbacks_after_unsubscribe = 0;
    struct FamilyStats {
        std::uint64_t callbacks = 0;
        std::uint64_t callbacks_after_unsubscribe = 0;
        std::uint64_t decode_errors = 0;
        std::uint64_t handler_exceptions = 0;
    };
    std::array<FamilyStats, families.size()> per_family{};
    BoundedQueue<Summary>::Counters queue;
    ::ams_gra::lacal::Stats stats;
    int runtime_threads = -1;          // live `ams-lacal*` threads after close()
    int sockets_at_start = -1;         // open socket fds when the client was created
    int sockets = -1;                  // open socket fds of this process after close()

    /// Every runtime thread joined and no socket left beyond the starting set.
    bool resources_released() const noexcept {
        return runtime_threads == 0 && sockets >= 0 && sockets_at_start >= 0 && sockets <= sockets_at_start;
    }
    bool clean() const noexcept {
        return unsubscribe_ok && close_ok && callbacks_after_unsubscribe == 0 && resources_released();
    }
};

class Client {
public:
    using Clock = std::chrono::steady_clock;
    using Queue = BoundedQueue<Summary>;

    explicit Client(ClientSettings settings, Extractor extract = summarize
#if CPP_UCI_CLIENT_WITH_RDMA
                    , RdmaExtractor extract_rdma = summarize_rdma
#endif
#if CPP_UCI_CLIENT_WITH_SURVIVABILITY
                    , SurvivabilityExtractor extract_survivability = summarize_survivability
#endif
                    )
        : settings_(std::move(settings)),
          receiver_(std::make_shared<Receiver>(settings_.queue_capacity, std::move(extract)
#if CPP_UCI_CLIENT_WITH_RDMA
                                               , std::move(extract_rdma)
#endif
#if CPP_UCI_CLIENT_WITH_SURVIVABILITY
                                               , std::move(extract_survivability)
#endif
                                               )),
          sockets_at_start_(count_open_sockets()) {}
    Client(const Client&) = delete;
    Client& operator=(const Client&) = delete;
    ~Client() { (void)shutdown(); }

    /// Runtime::connect with the build-time schema; nullopt on success.
    std::optional<Failure> connect() {
        if (runtime_ || shut_down_) {
            return Failure{exit_status::internal, "CONNECT-FAILED client already started or closed"};
        }
        ::ams_gra::lacal::Config config;
        config.url = settings_.url;
        config.service_id = settings_.service_id;
        config.schema_version = std::string(release::schema_version);  // never from the command line
        config.connect_timeout = settings_.connect_timeout;
        config.operation_timeout = settings_.operation_timeout;
        config.close_timeout = settings_.close_timeout;
        config.max_message_bytes = settings_.max_message_bytes;
        config.max_pending_deliveries = settings_.max_pending_deliveries;
        config.max_pending_delivery_bytes = settings_.max_pending_delivery_bytes;
        config.max_pending_events = settings_.max_pending_events;
        config.max_pending_event_bytes = settings_.max_pending_event_bytes;
        auto connected = ::ams_gra::lacal::Runtime::connect(std::move(config));
        if (!connected.ok()) {
            return Failure{connect_exit_status(connected.error()), "CONNECT-FAILED " + describe_error(connected.error())};
        }
        runtime_.emplace(std::move(connected).value());
        return std::nullopt;
    }

    /// INFO server_id (validated by the runtime), bounded for display.
    std::string server_id() const { return runtime_ ? info_server_id(runtime_->server_info()) : std::string(); }
    bool connected() const { return runtime_ && runtime_->connected(); }

    /// SUB through the GENERATED endpoint; returns after the broker's +OK/-ERR.
    std::optional<Failure> subscribe(Family family = Family::log) {
        if (!runtime_) {
            return Failure{exit_status::internal, "SUBSCRIBE-FAILED not connected"};
        }
        if (family == Family::survivability && !release::supports_survivability) {
            return Failure{exit_status::usage, "SUBSCRIBE-FAILED SurvivabilityRiskLevel is not built in"};
        }
        auto& slot = subscriptions_[index(family)];
        if (!slot.handle.id().empty()) {
            return Failure{exit_status::internal, "SUBSCRIBE-FAILED subscription already established"};
        }
        if (family == Family::log) {
            return retain(family, release::endpoint::subscribe(
                *runtime_, [receiver = receiver_](const release::m::LogMT& log) { receiver->on_log(log); }));
        }
#if CPP_UCI_CLIENT_WITH_SURVIVABILITY
        if (family == Family::survivability) {
            return retain(family, release::survivability_endpoint::subscribe(
                *runtime_, [receiver = receiver_](const release::m::SurvivabilityRiskLevelMT& risk) {
                    receiver->on_survivability(risk);
                }));
        }
#endif
#if CPP_UCI_CLIENT_WITH_RDMA
        return retain(family, release::rdma_endpoint::subscribe(
            *runtime_, [receiver = receiver_](const release::m::RDMAInitializeSetupMT& rdma) { receiver->on_rdma(rdma); }));
#else
        return Failure{exit_status::usage, "SUBSCRIBE-FAILED RDMA is not built in"};
#endif
    }

private:
    std::optional<Failure> retain(Family family, lacal::Result<lacal::Subscription> subscribed) {
        if (!subscribed.ok()) {
            const bool broker = subscribed.error().kind == lacal::ErrorKind::broker_rejected;
            return Failure{subscribe_exit_status(subscribed.error()),
                           std::string(broker ? "SUBSCRIBE-REJECTED by the broker: " : "SUBSCRIBE-FAILED ") +
                               describe_error(subscribed.error()) +
                               (family == Family::log ? "" : " message-name=" + std::string(message_name(family)))};
        }
        subscriptions_[index(family)].handle = std::move(subscribed).value();
        return std::nullopt;
    }

public:
    const std::string& subscription_id(Family family = Family::log) const noexcept {
        static const std::string absent;
        return index(family) < subscriptions_.size() ? subscriptions_[index(family)].handle.id() : absent;
    }
    Receiver& receiver() noexcept { return *receiver_; }

    /// Next typed summary (FIFO), runtime event, or idle, waiting at most until
    /// `deadline` (and at most ~25 ms, so signals and events stay responsive).
    /// Queued summaries are drained before events; both preserve their own order.
    Next next(Clock::time_point deadline) {
        Next result;
        if (!runtime_) {
            result.kind = Next::Kind::failure;
            result.exit_status = exit_status::internal;
            result.line = "INTERNAL not connected";
            return result;
        }
        const Clock::time_point now = Clock::now();
        if (take(now, result)) {
            return result;
        }
        if (auto event = runtime_->poll_event(std::chrono::milliseconds(0))) {
            EventOutcome outcome = record_event(*event);
            result.kind = outcome.fatal ? Next::Kind::failure : Next::Kind::notice;
            result.exit_status = exit_status::session;
            result.line = std::move(outcome.line);
            return result;
        }
        if (!runtime_->connected()) {
            result.kind = Next::Kind::failure;
            result.exit_status = exit_status::session;
            result.line = "CONNECTION-LOST the runtime reports the connection ended (no connection_closed event queued)";
            return result;
        }
        const Clock::time_point slice = now + std::chrono::milliseconds(25);
        (void)take(slice < deadline ? slice : deadline, result);
        return result;  // Kind::idle unless take() filled it in
    }

    /// A successful explicit UNSUB quiesces this SID only. On failed transport
    /// the runtime removes it locally without claiming an acknowledgment.
    bool unsubscribe(Family family, std::string& line) {
        if (index(family) >= subscriptions_.size()) {
            line.clear();
            return true;
        }
        auto& slot = subscriptions_[index(family)];
        if (slot.handle.active()) {
            const lacal::Status status = slot.handle.unsubscribe();
            if (status.ok()) {
                receiver_->mark_unsubscribed(family);
            }
            const std::string sid = safe_text(slot.handle.id(), 32);
            slot.ok = status.ok();
            slot.line = status.ok() ? "UNSUBSCRIBED sid=" + sid + " acknowledged by the broker (+OK)"
                                    : "UNSUBSCRIBE-FAILED sid=" + sid + " " + describe_error(status.error());
        }
        line = slot.line;
        return slot.ok;
    }

    /// UNSUB (if subscribed), close the queue (pending summaries are discarded
    /// and counted), drain events, close the runtime (joins its threads) and
    /// observe resources. Idempotent; main thread only.
    ShutdownReport shutdown() {
        if (shut_down_) {
            return report_;
        }
        shut_down_ = true;
        ShutdownReport report;
        report.session_failed = runtime_ && !runtime_->connected();
        for (auto it = families.rbegin(); it != families.rend(); ++it) {
            if (!subscription_id(*it).empty()) {
                report.subscribed = true;
                std::string line;
                const bool ok = unsubscribe(*it, line);
                report.unsubscribe_ok = report.unsubscribe_ok && ok;
                report.unsubscribe_lines.push_back(std::move(line));
            }
        }
        report.pending_discarded = receiver_->queue().close();
        report.close_line = "close=not-connected";
        if (runtime_) {
            report.connected_runtime = true;
            for (std::size_t i = 0; i <= settings_.max_pending_events; ++i) {
                auto event = runtime_->poll_event(std::chrono::milliseconds(0));
                if (!event) {
                    break;
                }
                report.notices.push_back(record_event(*event).line);
            }
            const ::ams_gra::lacal::Status closed = runtime_->close();
            report.close_ok = closed.ok();
            report.close_line = closed.ok() ? "close=ok" : "close=failed " + describe_error(closed.error());
            report.stats = runtime_->stats();
            runtime_.reset();
        }
        report.callbacks = receiver_->callbacks();
        report.callbacks_after_unsubscribe = receiver_->callbacks_after_unsubscribe();
        report.queue = receiver_->queue().counters();
        report.session_failed = report.session_failed || report.stats.handler_exceptions != 0 ||
                                report.queue.rejected_full != 0;
        report.per_family = per_family_;
        for (Family family : families) {
            auto& stats = report.per_family[index(family)];
            stats.callbacks = receiver_->callbacks(family);
            stats.callbacks_after_unsubscribe = receiver_->callbacks_after_unsubscribe(family);
        }
        report.runtime_threads = count_threads_named("ams-lacal");
        report.sockets_at_start = sockets_at_start_;
        report.sockets = count_open_sockets();
        report_ = report;
        return report;
    }

private:
    EventOutcome record_event(const lacal::Event& event) {
        for (Family family : families) {
            if (!event.sid.empty() && event.sid == subscription_id(family) && event.message_name == message_name(family)) {
                auto& stats = per_family_[index(family)];
                if (event.kind == lacal::EventKind::decode_error) {
                    ++stats.decode_errors;
                } else if (event.kind == lacal::EventKind::handler_exception) {
                    ++stats.handler_exceptions;
                }
                if (family == Family::survivability &&
                    (event.kind == lacal::EventKind::decode_error || event.kind == lacal::EventKind::handler_exception)) {
                    return describe_survivability_event(event);
                }
                break;
            }
        }
        if (event.message_name == release::survivability_message &&
            (event.kind == lacal::EventKind::decode_error || event.kind == lacal::EventKind::handler_exception)) {
            // Never fall back to another family's free-text diagnostic.
            return describe_survivability_event(event, false);
        }
        return describe_event(event);
    }

    bool take(Clock::time_point deadline, Next& result) {
        switch (receiver_->queue().pop_until(deadline, result.item.summary)) {
            case Queue::Pop::item:
                result.kind = Next::Kind::message;
                result.item.sequence = ++display_sequence_;
                return true;
            case Queue::Pop::overflowed: {
                const auto counters = receiver_->queue().counters();
                result.kind = Next::Kind::failure;
                result.exit_status = exit_status::session;
                result.line = "QUEUE-OVERFLOW application queue capacity=" +
                              std::to_string(receiver_->queue().capacity()) +
                              " exceeded; reception stopped instead of skipping " +
                              (subscription_id(Family::rdma).empty() ? "Log messages" : "typed messages") + " (accepted=" +
                              std::to_string(counters.accepted) + " rejected=" +
                              std::to_string(counters.rejected_full) + ")";
                return true;
            }
            case Queue::Pop::closed:
                result.kind = Next::Kind::failure;
                result.exit_status = exit_status::internal;
                result.line = "INTERNAL application queue already closed";
                return true;
            case Queue::Pop::timeout:
                return false;
        }
        return false;
    }

    ClientSettings settings_;
    std::shared_ptr<Receiver> receiver_;
    std::optional<::ams_gra::lacal::Runtime> runtime_;
    struct SubscriptionSlot {
        lacal::Subscription handle;
        bool ok = true;
        std::string line;
    };
    std::array<SubscriptionSlot, families.size()> subscriptions_{};
    std::array<ShutdownReport::FamilyStats, families.size()> per_family_{};
    std::uint64_t display_sequence_ = 0;
    bool shut_down_ = false;
    ShutdownReport report_;
    int sockets_at_start_ = -1;
};

}  // namespace cpp_uci_client
