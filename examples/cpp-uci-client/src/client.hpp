// CPP-OMS-008: the receiver core of the standalone C++ UCI Log client.
//
// LogClient composes the EXISTING C++ LA-CAL runtime (ams_gra::lacal::Runtime)
// with the production-GENERATED receive endpoint and codec. It adds no
// transport, codec or protocol logic of its own:
//
//   connect()    Runtime::connect: TCP, WebSocket upgrade offering `owp`
//                (response must select exactly `owp`), verbose INIT, +OK, INFO.
//   subscribe()  service_api::function_log::exchange_receive::subscribe: SUB
//                Log uci.log; returns only after the broker's +OK / -ERR.
//   next()       one typed Log (FIFO), one runtime event, or idle; bounded wait.
//   shutdown()   UNSUB (acknowledged), close the queue, Runtime::close() (joins
//                the runtime's threads); idempotent; main thread only.
//
// Every method is called from ONE application thread. The typed callback runs
// on the runtime's dispatcher thread and touches only `Receiver` (atomics and
// the bounded queue). No application lock is held while a runtime operation
// runs, and the runtime is never closed or destroyed from its handler.
#pragma once

#include <atomic>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <optional>
#include <string>
#include <thread>
#include <utility>
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

namespace cpp_uci_client {

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

struct Delivered {
    std::uint64_t sequence = 0;  // 1-based callback order within the subscription
    LogSummary summary;
};

using Extractor = std::function<LogSummary(const ::programs::oam::LogMT&)>;

/// The only state the runtime's dispatcher thread touches.
class Receiver {
public:
    Receiver(std::size_t capacity, Extractor extract) : queue_(capacity), extract_(std::move(extract)) {}

    /// The typed callback body: copy the selected fields, hand off, return.
    void on_log(const ::programs::oam::LogMT& log) {
        const std::uint64_t sequence = callbacks_.fetch_add(1, std::memory_order_acq_rel) + 1;
        if (unsubscribed_.load(std::memory_order_acquire)) {
            callbacks_after_unsubscribe_.fetch_add(1, std::memory_order_acq_rel);
        }
        (void)queue_.push(Delivered{sequence, extract_(log)});  // never blocks; full => sticky overflow
    }

    BoundedQueue<Delivered>& queue() noexcept { return queue_; }
    void mark_unsubscribed() noexcept { unsubscribed_.store(true, std::memory_order_release); }
    std::uint64_t callbacks() const noexcept { return callbacks_.load(std::memory_order_acquire); }
    std::uint64_t callbacks_after_unsubscribe() const noexcept {
        return callbacks_after_unsubscribe_.load(std::memory_order_acquire);
    }

private:
    BoundedQueue<Delivered> queue_;
    Extractor extract_;
    std::atomic<std::uint64_t> callbacks_{0};
    std::atomic<bool> unsubscribed_{false};
    std::atomic<std::uint64_t> callbacks_after_unsubscribe_{0};
};

struct Failure {
    int exit_status = exit_status::internal;
    std::string line;
};

struct Next {
    enum class Kind { log, notice, idle, failure };
    Kind kind = Kind::idle;
    Delivered item;       // Kind::log
    std::string line;     // Kind::notice / Kind::failure
    int exit_status = 0;  // Kind::failure
};

struct ShutdownReport {
    bool subscribed = false;
    bool unsubscribe_ok = true;
    std::string unsubscribe_line;
    bool connected_runtime = false;
    bool close_ok = true;
    std::string close_line;
    std::vector<std::string> notices;  // events drained during shutdown
    std::size_t pending_discarded = 0; // delivered to the queue, never displayed
    std::uint64_t callbacks = 0;
    std::uint64_t callbacks_after_unsubscribe = 0;
    BoundedQueue<Delivered>::Counters queue;
    ::ams_gra::lacal::Stats stats;
    int runtime_threads = -1;          // live `ams-lacal*` threads after close()
    int sockets_at_start = -1;         // open socket fds when the LogClient was created
    int sockets = -1;                  // open socket fds of this process after close()

    /// Every runtime thread joined and no socket left beyond the starting set.
    bool resources_released() const noexcept {
        return runtime_threads == 0 && sockets >= 0 && sockets_at_start >= 0 && sockets <= sockets_at_start;
    }
    bool clean() const noexcept {
        return unsubscribe_ok && close_ok && callbacks_after_unsubscribe == 0 && resources_released();
    }
};

class LogClient {
public:
    using Clock = std::chrono::steady_clock;
    using Queue = BoundedQueue<Delivered>;

    explicit LogClient(ClientSettings settings, Extractor extract = summarize)
        : settings_(std::move(settings)),
          receiver_(std::make_shared<Receiver>(settings_.queue_capacity, std::move(extract))),
          sockets_at_start_(count_open_sockets()) {}
    LogClient(const LogClient&) = delete;
    LogClient& operator=(const LogClient&) = delete;
    ~LogClient() { (void)shutdown(); }

    /// Runtime::connect with the build-time schema; nullopt on success.
    std::optional<Failure> connect() {
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

    /// SUB through the GENERATED endpoint; returns after the broker's +OK/-ERR.
    std::optional<Failure> subscribe() {
        if (!runtime_) {
            return Failure{exit_status::internal, "SUBSCRIBE-FAILED not connected"};
        }
        std::shared_ptr<Receiver> receiver = receiver_;
        auto subscribed = ::service_api::function_log::exchange_receive::subscribe(
            *runtime_, [receiver](const ::programs::oam::LogMT& log) { receiver->on_log(log); });
        if (!subscribed.ok()) {
            const bool broker = subscribed.error().kind == ::ams_gra::lacal::ErrorKind::broker_rejected;
            return Failure{subscribe_exit_status(subscribed.error()),
                           std::string(broker ? "SUBSCRIBE-REJECTED by the broker: " : "SUBSCRIBE-FAILED ") +
                               describe_error(subscribed.error())};
        }
        subscription_ = std::move(subscribed).value();
        sid_ = subscription_.id();
        return std::nullopt;
    }

    const std::string& subscription_id() const noexcept { return sid_; }
    Receiver& receiver() noexcept { return *receiver_; }

    /// Next typed Log (FIFO), runtime event, or idle, waiting at most until
    /// `deadline` (and at most ~25 ms, so signals and events stay responsive).
    /// Queued Logs are drained before events; both preserve their own order.
    Next next(Clock::time_point deadline) {
        Next result;
        if (!runtime_) {
            result.kind = Next::Kind::failure;
            result.exit_status = exit_status::internal;
            result.line = "INTERNAL not connected";
            return result;
        }
        Delivered item;
        const Clock::time_point now = Clock::now();
        if (take(receiver_->queue().pop_until(now, item), item, result)) {
            return result;
        }
        if (auto event = runtime_->poll_event(std::chrono::milliseconds(0))) {
            EventOutcome outcome = describe_event(*event);
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
        (void)take(receiver_->queue().pop_until(slice < deadline ? slice : deadline, item), item, result);
        return result;  // Kind::idle unless take() filled it in
    }

    /// Explicit UNSUB (acknowledged). After it returns, the runtime guarantees
    /// the callback is neither running nor invoked again.
    bool unsubscribe(std::string& line) {
        if (!subscription_.active()) {
            line = unsubscribe_line_;
            return unsubscribe_ok_;
        }
        const ::ams_gra::lacal::Status status = subscription_.unsubscribe();
        receiver_->mark_unsubscribed();
        unsubscribe_ok_ = status.ok();
        unsubscribe_line_ = status.ok()
                                ? "UNSUBSCRIBED sid=" + safe_text(sid_, 32) + " acknowledged by the broker (+OK)"
                                : "UNSUBSCRIBE-FAILED sid=" + safe_text(sid_, 32) + " " + describe_error(status.error());
        line = unsubscribe_line_;
        return unsubscribe_ok_;
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
        report.subscribed = !sid_.empty();
        if (report.subscribed) {
            report.unsubscribe_ok = unsubscribe(report.unsubscribe_line);
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
                report.notices.push_back(describe_event(*event).line);
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
        report.runtime_threads = count_threads_named("ams-lacal");
        report.sockets_at_start = sockets_at_start_;
        report.sockets = count_open_sockets();
        report_ = report;
        return report;
    }

private:
    bool take(Queue::Pop pop, Delivered& item, Next& result) {
        switch (pop) {
            case Queue::Pop::item:
                result.kind = Next::Kind::log;
                result.item = std::move(item);
                return true;
            case Queue::Pop::overflowed: {
                const auto counters = receiver_->queue().counters();
                result.kind = Next::Kind::failure;
                result.exit_status = exit_status::session;
                result.line = "QUEUE-OVERFLOW application queue capacity=" +
                              std::to_string(receiver_->queue().capacity()) +
                              " exceeded; reception stopped instead of skipping Log messages (accepted=" +
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
    ::ams_gra::lacal::Subscription subscription_;
    std::string sid_;
    bool unsubscribe_ok_ = true;
    std::string unsubscribe_line_;
    bool shut_down_ = false;
    ShutdownReport report_;
    int sockets_at_start_ = -1;
};

}  // namespace cpp_uci_client
