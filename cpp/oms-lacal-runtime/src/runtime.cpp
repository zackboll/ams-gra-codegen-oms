// CPP-OMS-003: LA-CAL runtime implementation. See runtime.hpp for the
// contract and docs/task-cpp-oms-003-lacal-runtime.md for the design.
//
// Ownership and locking rules (checked by tests/cpp-lacal):
//  * The I/O thread owns `io_->ws`, the resolver, timers, the read buffer and the
//    write queue. No other thread touches them; other threads `post` work.
//  * `mutex_` guards every field shared between threads (state, pending
//    command, subscriptions, inbound backlog, events, stats). It is never held
//    while a handler, a codec, a user destructor or a blocking wait other than
//    a condition variable runs.
//  * The command slot (`command_busy_`) serialises acknowledged commands: at most one OWP
//    command awaits +OK/-ERR at a time, so every status is paired with the
//    command that caused it. Waits on it and for the status are bounded.
//  * Both threads are joined by close(); Core is never destroyed while either
//    runs, and handlers capture `this` only for work that completes before
//    the I/O context is destroyed.
#include "ams_gra/lacal/runtime.hpp"

#include <boost/version.hpp>
static_assert(BOOST_VERSION >= 108100, "CPP-OMS-003 requires Boost.Beast from Boost 1.81 or newer");

#include <boost/asio/ip/tcp.hpp>
#include <boost/asio/post.hpp>
#include <boost/asio/steady_timer.hpp>
#include <boost/beast/core.hpp>
#include <boost/beast/http.hpp>
#include <boost/beast/websocket.hpp>

#include <algorithm>
#include <atomic>
#include <cctype>
#include <condition_variable>
#include <cstdio>
#include <deque>
#include <functional>
#include <limits>
#include <map>
#include <mutex>
#include <thread>

#if defined(__linux__)
#include <pthread.h>
#endif

namespace ams_gra::lacal {

namespace net = boost::asio;
namespace beast = boost::beast;
namespace http = beast::http;
namespace websocket = beast::websocket;
using tcp = net::ip::tcp;

const char* to_string(ErrorKind kind) noexcept {
    switch (kind) {
        case ErrorKind::none: return "none";
        case ErrorKind::invalid_argument: return "invalid_argument";
        case ErrorKind::unsupported_scheme: return "unsupported_scheme";
        case ErrorKind::codec: return "codec";
        case ErrorKind::connect_failed: return "connect_failed";
        case ErrorKind::handshake_rejected: return "handshake_rejected";
        case ErrorKind::broker_rejected: return "broker_rejected";
        case ErrorKind::protocol_violation: return "protocol_violation";
        case ErrorKind::timeout: return "timeout";
        case ErrorKind::connection_closed: return "connection_closed";
        case ErrorKind::subscription_inactive: return "subscription_inactive";
        case ErrorKind::subscription_ids_exhausted: return "subscription_ids_exhausted";
        case ErrorKind::delivery_overflow: return "delivery_overflow";
        case ErrorKind::called_from_runtime_thread: return "called_from_runtime_thread";
    }
    return "unknown";
}

const char* to_string(EventKind kind) noexcept {
    switch (kind) {
        case EventKind::server_error: return "server_error";
        case EventKind::decode_error: return "decode_error";
        case EventKind::unknown_subscription: return "unknown_subscription";
        case EventKind::handler_exception: return "handler_exception";
        case EventKind::info: return "info";
        case EventKind::connection_closed: return "connection_closed";
        case EventKind::events_dropped: return "events_dropped";
    }
    return "unknown";
}

namespace detail {

namespace {

Error make_error(ErrorKind kind, std::string message) {
    Error error;
    error.kind = kind;
    error.message = std::move(message);
    return error;
}

/// Names a runtime thread from its creator, so the name is visible as soon
/// as the creating call returns (diagnostics, leak checks in tests).
void name_thread(std::thread& thread, const char* name) {
#if defined(__linux__)
    pthread_setname_np(thread.native_handle(), name);
#else
    (void)thread;
    (void)name;
#endif
}

struct Endpoint {
    std::string host;         // for DNS / connect (brackets removed)
    std::string port;
    std::string host_header;  // Host header value
    std::string target;       // request target, "/" by default
};

std::string lower(std::string_view text) {
    std::string out(text);
    for (char& c : out) {
        c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    }
    return out;
}

/// ws://host[:port][/path][?query]. Anything else is refused explicitly.
Result<Endpoint> parse_url(std::string_view url) {
    const std::size_t colon = url.find("://");
    if (colon == std::string_view::npos) {
        return Result<Endpoint>::failure(make_error(ErrorKind::invalid_argument, "URL has no scheme: '" + std::string(url) + "'"));
    }
    const std::string scheme = lower(url.substr(0, colon));
    if (scheme == "wss") {
        return Result<Endpoint>::failure(make_error(
            ErrorKind::unsupported_scheme,
            "wss:// (TLS) is not implemented by this runtime; only loopback-qualified ws:// is supported"));
    }
    if (scheme != "ws") {
        return Result<Endpoint>::failure(
            make_error(ErrorKind::unsupported_scheme, "unsupported URL scheme '" + scheme + "://'; expected ws://"));
    }
    std::string_view rest = url.substr(colon + 3);
    if (rest.find('#') != std::string_view::npos) {
        return Result<Endpoint>::failure(make_error(ErrorKind::invalid_argument, "URL fragment is not allowed"));
    }
    const std::size_t slash = rest.find_first_of("/?");
    std::string_view authority = rest.substr(0, slash);
    std::string target = slash == std::string_view::npos ? "/" : std::string(rest.substr(slash));
    if (!target.empty() && target.front() == '?') {
        target.insert(target.begin(), '/');
    }
    if (authority.empty() || authority.find('@') != std::string_view::npos) {
        return Result<Endpoint>::failure(
            make_error(ErrorKind::invalid_argument, "URL authority must be host[:port] without user information"));
    }
    Endpoint endpoint;
    endpoint.host_header = std::string(authority);
    std::string_view port;
    if (authority.front() == '[') {
        const std::size_t close = authority.find(']');
        if (close == std::string_view::npos) {
            return Result<Endpoint>::failure(make_error(ErrorKind::invalid_argument, "unterminated IPv6 literal in URL"));
        }
        endpoint.host = std::string(authority.substr(1, close - 1));
        std::string_view after = authority.substr(close + 1);
        if (!after.empty()) {
            if (after.front() != ':') {
                return Result<Endpoint>::failure(make_error(ErrorKind::invalid_argument, "invalid URL authority"));
            }
            port = after.substr(1);
        }
    } else {
        const std::size_t port_colon = authority.rfind(':');
        endpoint.host = std::string(authority.substr(0, port_colon));
        if (port_colon != std::string_view::npos) {
            port = authority.substr(port_colon + 1);
        }
    }
    if (endpoint.host.empty()) {
        return Result<Endpoint>::failure(make_error(ErrorKind::invalid_argument, "URL host is empty"));
    }
    if (port.empty()) {
        endpoint.port = "80";
    } else {
        unsigned long value = 0;
        for (const char c : port) {
            if (c < '0' || c > '9' || value > 65535UL) {
                return Result<Endpoint>::failure(make_error(ErrorKind::invalid_argument, "invalid URL port"));
            }
            value = value * 10UL + static_cast<unsigned long>(c - '0');
        }
        if (value == 0 || value > 65535UL) {
            return Result<Endpoint>::failure(make_error(ErrorKind::invalid_argument, "invalid URL port"));
        }
        endpoint.port = std::string(port);
    }
    endpoint.target = std::move(target);
    return Result<Endpoint>::success(std::move(endpoint));
}

struct SubEntry {
    std::string sid;
    std::string element;
    std::unique_ptr<Delivery> delivery;
    bool active = false;   // dispatchable: set by the I/O thread on the SUB's +OK
    bool running = false;  // the dispatcher is inside this entry's delivery
};

struct Pending {
    bool done = false;
    Error error;                         // kind none == +OK
    std::function<void(bool)> on_ack;    // runs on the I/O thread, under mutex_, at the ack
};

struct Inbound {
    std::string sid;
    std::string payload;
};

}  // namespace

class Core {
public:
    Core(Config config, Endpoint endpoint)
        : config_(std::move(config)), endpoint_(std::move(endpoint)), io_(std::make_unique<Io>()) {}

    Core(const Core&) = delete;
    Core& operator=(const Core&) = delete;

    ~Core() {
        if (io_thread_.joinable() || dispatch_thread_.joinable()) {
            std::fputs("ams_gra::lacal: runtime core destroyed with running threads\n", stderr);
            std::terminate();
        }
    }

    // ---- lifecycle (application threads) ------------------------------------

    Status start() {
        io_thread_ = std::thread([this] { io_main(); });
        name_thread(io_thread_, "ams-lacal-io");
        io_id_.store(io_thread_.get_id());
        net::post(io_->ioc, [this] { begin_connect(); });
        std::unique_lock<std::mutex> lock(mutex_);
        const auto deadline = std::chrono::steady_clock::now() + config_.connect_timeout + std::chrono::seconds(2);
        const bool finished = cv_.wait_until(lock, deadline, [this] { return connect_finished_; });
        if (!finished) {
            // The I/O-side timer should already have fired; never wait longer.
            connect_finished_ = true;
            connect_error_ = make_error(ErrorKind::timeout, "connect did not complete within connect_timeout");
            state_ = State::failed;
        }
        if (connect_error_.kind != ErrorKind::none) {
            Error error = connect_error_;
            lock.unlock();
            stop_io();
            return Status::failure(std::move(error));
        }
        lock.unlock();
        dispatch_thread_ = std::thread([this] { dispatch_main(); });
        name_thread(dispatch_thread_, "ams-lacal-dsp");
        dispatch_id_.store(dispatch_thread_.get_id());
        return Status::success();
    }

    Status close() {
        if (on_runtime_thread()) {
            return Status::failure(make_error(ErrorKind::called_from_runtime_thread,
                                              "close() must not be called from a runtime handler"));
        }
        std::lock_guard<std::mutex> serial(close_mutex_);
        if (joined_) {
            return Status::success();
        }
        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (state_ == State::active) {
                Error reason = make_error(ErrorKind::connection_closed, "runtime closed by the application");
                end_locked(reason);
            }
            state_ = State::closed;
        }
        net::post(io_->ioc, [this] { begin_close(); });
        stop_io();
        std::map<std::string, std::shared_ptr<SubEntry>> released;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            stop_dispatch_ = true;
        }
        dispatch_cv_.notify_all();
        if (dispatch_thread_.joinable()) {
            dispatch_thread_.join();
        }
        std::unique_ptr<Io> io;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            released.swap(subs_);
            io.swap(io_);
            joined_ = true;
        }
        sub_cv_.notify_all();
        released.clear();  // user handlers are destroyed here, with no lock held
        io.reset();        // destroys the Asio objects; joins Asio's resolver thread
        return Status::success();
    }

    // ---- operations (application or dispatcher threads) ----------------------

    Status publish(std::string element, std::string_view topic, const std::string& body) {
        if (!owp::is_identifier(topic)) {
            return Status::failure(make_error(ErrorKind::invalid_argument,
                                              "topic '" + std::string(topic) + "' is not an OWP identifier"));
        }
        return command(owp::build_pub(topic, element, body), nullptr);
    }

    Result<std::string> subscribe(std::string element, std::string_view topic,
                                  std::optional<std::string_view> group,
                                  std::unique_ptr<Delivery> delivery) {
        std::string sid;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (next_sid_ == std::numeric_limits<std::uint64_t>::max()) {
                return Result<std::string>::failure(make_error(ErrorKind::subscription_ids_exhausted,
                                                               "subscription id counter exhausted"));
            }
            sid = "sub-" + std::to_string(next_sid_++);  // consumed even if rejected below
        }
        if (!owp::is_identifier(topic)) {
            return Result<std::string>::failure(make_error(
                ErrorKind::invalid_argument, "topic '" + std::string(topic) + "' is not an OWP identifier"));
        }
        if (group && !owp::is_identifier(*group)) {
            return Result<std::string>::failure(make_error(
                ErrorKind::invalid_argument, "subscription group '" + std::string(*group) + "' is not an OWP identifier"));
        }
        auto entry = std::make_shared<SubEntry>();
        entry->sid = sid;
        entry->element = element;
        entry->delivery = std::move(delivery);
        {
            std::lock_guard<std::mutex> lock(mutex_);
            subs_.emplace(sid, entry);
        }
        std::weak_ptr<SubEntry> weak = entry;
        Status status = command(owp::build_sub(sid, element, topic, group), [weak](bool ok) {
            if (auto target = weak.lock(); target && ok) {
                target->active = true;
            }
        });
        if (!status.ok()) {
            std::shared_ptr<SubEntry> removed;
            {
                std::lock_guard<std::mutex> lock(mutex_);
                auto it = subs_.find(sid);
                if (it != subs_.end() && !it->second->active) {
                    removed = std::move(it->second);
                    subs_.erase(it);
                }
            }
            entry.reset();
            removed.reset();  // handler destroyed without any lock held
            return Result<std::string>::failure(status.error());
        }
        return Result<std::string>::success(std::move(sid));
    }

    Status unsubscribe(const std::string& sid) {
        if (on_io_thread()) {
            return Status::failure(make_error(ErrorKind::called_from_runtime_thread, "unsubscribe from the I/O thread"));
        }
        std::shared_ptr<SubEntry> entry;
        bool handler_still_running = false;
        bool connection_usable = false;
        Error closed_reason;
        {
            std::unique_lock<std::mutex> lock(mutex_);
            if (state_ == State::closed) {
                return Status::failure(make_error(ErrorKind::connection_closed, "runtime is closed"));
            }
            auto it = subs_.find(sid);
            if (it == subs_.end() || !it->second->active) {
                return Status::failure(make_error(ErrorKind::subscription_inactive,
                                                  "subscription '" + sid + "' is not active"));
            }
            entry = std::move(it->second);
            subs_.erase(it);  // no later MSG for this SID reaches the handler
            entry->active = false;
            if (!on_dispatch_thread()) {
                handler_still_running = !sub_cv_.wait_for(lock, config_.operation_timeout,
                                                          [&] { return !entry->running; });
            }
            connection_usable = state_ == State::active;
            closed_reason = failure_;
        }
        Status status = Status::success();
        if (!connection_usable) {
            status = Status::failure(make_error(
                ErrorKind::connection_closed,
                "subscription removed locally; UNSUB not sent because the connection ended: " + closed_reason.message));
        } else {
            status = command(owp::build_unsub(sid), nullptr);
        }
        entry.reset();  // may destroy the handler: no lock held
        if (status.ok() && handler_still_running) {
            return Status::failure(make_error(ErrorKind::timeout,
                                              "UNSUB acknowledged, but the handler was still running after operation_timeout"));
        }
        return status;
    }

    std::optional<Event> poll_event(std::chrono::milliseconds timeout) {
        std::unique_lock<std::mutex> lock(mutex_);
        event_cv_.wait_for(lock, timeout, [this] { return !events_.empty() || dropped_unreported_ != 0; });
        if (dropped_unreported_ != 0) {
            Event event;
            event.kind = EventKind::events_dropped;
            event.count = dropped_unreported_;
            dropped_unreported_ = 0;
            return event;
        }
        if (events_.empty()) {
            return std::nullopt;
        }
        Event event = std::move(events_.front());
        events_.pop_front();
        return event;
    }

    bool connected() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return state_ == State::active;
    }

    Stats stats() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return stats_;
    }

    const std::string& server_info() const { return info_; }  // written once before start() returns

    bool on_runtime_thread() const { return on_io_thread() || on_dispatch_thread(); }

private:
    enum class State { connecting, active, failed, closed };
    enum class Phase { connecting, awaiting_ok, awaiting_info, active, ended };

    bool on_io_thread() const { return std::this_thread::get_id() == io_id_.load(); }
    bool on_dispatch_thread() const { return std::this_thread::get_id() == dispatch_id_.load(); }

    // ---- acknowledged command ------------------------------------------------

    Status command(std::string frame, std::function<void(bool)> on_ack) {
        if (on_io_thread()) {
            return Status::failure(make_error(ErrorKind::called_from_runtime_thread, "command from the I/O thread"));
        }
        CommandSlot slot(*this);
        if (!slot.acquired) {
            return Status::failure(make_error(ErrorKind::timeout,
                                              "another operation was still awaiting acknowledgment; nothing was sent"));
        }
        auto pending = std::make_shared<Pending>();
        pending->on_ack = std::move(on_ack);
        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (state_ != State::active) {
                return Status::failure(closed_error_locked());
            }
            pending_ = pending;
            // Posted under mutex_: close() destroys io_ only after it has
            // observed State::closed under the same lock.
            net::post(io_->ioc, [this, frame = std::move(frame)]() mutable { send(std::move(frame)); });
        }
        std::unique_lock<std::mutex> lock(mutex_);
        const bool done = cv_.wait_for(lock, config_.operation_timeout, [&] { return pending->done; });
        if (!done) {
            // A late +OK/-ERR could no longer be paired with its command, so
            // the connection is failed rather than risking a wrong pairing.
            Error error = make_error(ErrorKind::timeout,
                                     "no +OK/-ERR within operation_timeout; connection closed because a late "
                                     "acknowledgment could not be paired with its command");
            pending->done = true;
            pending->error = error;
            if (pending_ == pending) {
                pending_.reset();
            }
            end_locked(error);
            if (state_ != State::closed) {
                net::post(io_->ioc, [this] { abort_transport(websocket::close_code::policy_error); });
            }
            return Status::failure(std::move(error));
        }
        if (pending->error.kind != ErrorKind::none) {
            return Status::failure(pending->error);
        }
        return Status::success();
    }

    /// Bounded exclusive right to have one command awaiting its +OK/-ERR.
    /// (A mutex/condition-variable pair rather than std::timed_mutex, so the
    /// wait is bounded and fully visible to ThreadSanitizer.)
    struct CommandSlot {
        explicit CommandSlot(Core& core) : owner(core) {
            std::unique_lock<std::mutex> lock(owner.mutex_);
            acquired = owner.slot_cv_.wait_for(lock, owner.config_.operation_timeout,
                                               [this] { return !owner.command_busy_; });
            if (acquired) {
                owner.command_busy_ = true;
            }
        }
        ~CommandSlot() {
            if (acquired) {
                {
                    std::lock_guard<std::mutex> lock(owner.mutex_);
                    owner.command_busy_ = false;
                }
                owner.slot_cv_.notify_one();
            }
        }
        CommandSlot(const CommandSlot&) = delete;
        CommandSlot& operator=(const CommandSlot&) = delete;
        Core& owner;
        bool acquired = false;
    };

    Error closed_error_locked() const {
        if (state_ == State::closed) {
            return make_error(ErrorKind::connection_closed, "runtime is closed");
        }
        Error error = failure_;
        if (error.kind == ErrorKind::none) {
            error = make_error(ErrorKind::connection_closed, "not connected");
        }
        return make_error(ErrorKind::connection_closed, "connection ended: " + error.message);
    }

    /// Marks the connection ended (once): fails the outstanding command and
    /// queues a connection_closed event. Caller holds mutex_.
    void end_locked(const Error& reason) {
        if (state_ == State::failed || state_ == State::closed) {
            return;
        }
        state_ = State::failed;
        failure_ = reason;
        if (pending_ && !pending_->done) {
            pending_->done = true;
            pending_->error = make_error(reason.kind == ErrorKind::timeout ? ErrorKind::timeout : ErrorKind::connection_closed,
                                         "connection ended before acknowledgment: " + reason.message);
            if (reason.kind == ErrorKind::delivery_overflow || reason.kind == ErrorKind::protocol_violation) {
                pending_->error.kind = reason.kind;
            }
        }
        pending_.reset();
        Event event;
        event.kind = EventKind::connection_closed;
        event.error = reason;
        push_event_locked(std::move(event));
        cv_.notify_all();
    }

    void push_event_locked(Event event) {
        if (events_.size() >= config_.max_pending_events) {
            ++stats_.events_dropped;
            ++dropped_unreported_;
        } else {
            events_.push_back(std::move(event));
        }
        event_cv_.notify_all();
    }

    // ---- I/O thread ----------------------------------------------------------

    void io_main() {
        for (;;) {
            try {
                io_->ioc.run();
                break;
            } catch (const std::exception& error) {
                // Completion handlers do not throw by design; contain anything
                // unexpected and keep the context alive until close().
                std::lock_guard<std::mutex> lock(mutex_);
                end_locked(make_error(ErrorKind::protocol_violation,
                                      std::string("internal I/O error: ") + error.what()));
            }
        }
        std::lock_guard<std::mutex> lock(mutex_);
        io_exited_ = true;
        cv_.notify_all();
    }

    void stop_io() {
        net::post(io_->ioc, [this] { io_->work.reset(); });
        {
            std::unique_lock<std::mutex> lock(mutex_);
            const auto bound = config_.close_timeout + std::chrono::seconds(2);
            if (!cv_.wait_for(lock, bound, [this] { return io_exited_; })) {
                io_->ioc.stop();  // bounded: abandon any I/O still outstanding
            }
        }
        if (io_thread_.joinable()) {
            io_thread_.join();
        }
    }

    void begin_connect() {
        io_->connect_timer.expires_after(config_.connect_timeout);
        io_->connect_timer.async_wait([this](beast::error_code ec) {
            if (!ec && phase_ != Phase::active && phase_ != Phase::ended) {
                connect_timed_out_ = true;
                io_->resolver.cancel();
                beast::get_lowest_layer(io_->ws).close();
            }
        });
        beast::error_code literal_ec;
        const net::ip::address literal = net::ip::make_address(endpoint_.host, literal_ec);
        if (!literal_ec) {
            // IP literal: no DNS, so Asio starts no resolver thread.
            const tcp::endpoint target(literal, static_cast<unsigned short>(std::stoul(endpoint_.port)));
            beast::get_lowest_layer(io_->ws).async_connect(
                target, [this](beast::error_code connect_ec) { on_tcp_connect(connect_ec); });
            return;
        }
        io_->resolver.async_resolve(endpoint_.host, endpoint_.port,
                                [this](beast::error_code ec, tcp::resolver::results_type results) {
                                    if (ec) {
                                        return connect_failed(ErrorKind::connect_failed, "resolve failed: " + ec.message());
                                    }
                                    beast::get_lowest_layer(io_->ws).async_connect(
                                        results, [this](beast::error_code connect_ec, const tcp::endpoint&) {
                                            on_tcp_connect(connect_ec);
                                        });
                                });
    }

    void on_tcp_connect(beast::error_code ec) {
        if (ec) {
            return connect_failed(ErrorKind::connect_failed, "TCP connect failed: " + ec.message());
        }
        beast::error_code ignored;
        // OWP operations are small request/acknowledgment frames: no Nagle delay.
        beast::get_lowest_layer(io_->ws).socket().set_option(tcp::no_delay(true), ignored);
        io_->ws.set_option(websocket::stream_base::decorator([](websocket::request_type& request) {
            request.set(http::field::sec_websocket_protocol, "owp");
            request.set(http::field::user_agent, "ams-gra-oms-lacal-cpp/0.1");
        }));
        io_->ws.async_handshake(handshake_response_, endpoint_.host_header, endpoint_.target,
                            [this](beast::error_code handshake_ec) { on_handshake(handshake_ec); });
    }

    void on_handshake(beast::error_code ec) {
        if (ec == websocket::error::upgrade_declined) {
            return connect_failed(ErrorKind::handshake_rejected,
                                  "WebSocket upgrade declined with HTTP " +
                                      std::to_string(handshake_response_.result_int()));
        }
        if (ec) {
            return connect_failed(ErrorKind::connect_failed, "WebSocket handshake failed: " + ec.message());
        }
        const auto protocol = handshake_response_[http::field::sec_websocket_protocol];
        if (protocol != "owp") {
            return connect_failed(ErrorKind::handshake_rejected,
                                  "server did not select subprotocol 'owp' (got '" + std::string(protocol) + "')");
        }
        beast::get_lowest_layer(io_->ws).expires_never();
        websocket::stream_base::timeout timeouts{};
        timeouts.handshake_timeout = config_.close_timeout;  // also bounds the close handshake
        timeouts.idle_timeout = websocket::stream_base::none();
        timeouts.keep_alive_pings = false;
        io_->ws.set_option(timeouts);
        io_->ws.read_message_max(config_.max_message_bytes);
        io_->ws.text(true);
        phase_ = Phase::awaiting_ok;
        send(owp::build_init(config_.owp_versions, config_.schema_version, config_.service_id, true));
        read_next();
    }

    void connect_failed(ErrorKind kind, std::string message) { connect_failed(make_error(kind, std::move(message))); }

    void connect_failed(Error error) {
        if (phase_ == Phase::ended) {
            return;
        }
        if (connect_timed_out_) {
            error.kind = ErrorKind::timeout;
            error.message = "connect did not complete within connect_timeout (" + error.message + ")";
        }
        phase_ = Phase::ended;
        io_->connect_timer.cancel();
        abort_transport(websocket::close_code::normal);
        std::lock_guard<std::mutex> lock(mutex_);
        connect_finished_ = true;
        connect_error_ = error;
        state_ = State::failed;
        failure_ = std::move(error);
        cv_.notify_all();
    }

    void send(std::string frame) {
        if (phase_ == Phase::ended) {
            return;
        }
        write_queue_.push_back(std::move(frame));
        if (!writing_) {
            write_next();
        }
    }

    void write_next() {
        writing_ = true;
        io_->ws.async_write(net::buffer(write_queue_.front()), [this](beast::error_code ec, std::size_t) {
            if (ec) {
                writing_ = false;
                write_queue_.clear();
                return transport_lost("write failed: " + ec.message());
            }
            write_queue_.pop_front();
            if (write_queue_.empty() || phase_ == Phase::ended) {
                writing_ = false;
                if (phase_ == Phase::ended) {
                    write_queue_.clear();
                }
                return;
            }
            write_next();
        });
    }

    void read_next() {
        io_->ws.async_read(read_buffer_, [this](beast::error_code ec, std::size_t) { on_read(ec); });
    }

    void on_read(beast::error_code ec) {
        if (ec) {
            if (ec == websocket::error::closed) {
                const auto& reason = io_->ws.reason();
                return transport_lost("peer closed the WebSocket (code " + std::to_string(reason.code) +
                                      (reason.reason.empty() ? std::string() : ", \"" + std::string(reason.reason.c_str()) + "\"") + ")");
            }
            if (ec == websocket::error::message_too_big || ec == websocket::error::buffer_overflow) {
                return protocol_failure("incoming message exceeds max_message_bytes");
            }
            return transport_lost("read failed: " + ec.message());
        }
        if (!io_->ws.got_text()) {
            read_buffer_.consume(read_buffer_.size());
            return protocol_failure("binary WebSocket message (OWP uses text messages only)");
        }
        std::string frame = beast::buffers_to_string(read_buffer_.data());
        read_buffer_.consume(read_buffer_.size());
        if (handle_frame(frame) && phase_ != Phase::ended) {
            read_next();
        }
    }

    /// Returns false when the connection was ended by this frame.
    bool handle_frame(const std::string& text) {
        owp::ServerFrame frame = owp::parse_server_frame(text);
        using Kind = owp::ServerFrame::Kind;
        if (frame.kind == Kind::invalid) {
            protocol_failure("invalid OWP frame: " + frame.problem);
            return false;
        }
        switch (phase_) {
            case Phase::awaiting_ok:
                if (frame.kind == Kind::ok) {
                    phase_ = Phase::awaiting_info;
                    return true;
                }
                if (frame.kind == Kind::err) {
                    Error error = make_error(ErrorKind::broker_rejected,
                                             "INIT rejected: -ERR " + frame.code + (frame.details.empty() ? "" : " " + frame.details));
                    error.owp_code = frame.code;
                    error.owp_details = frame.details;
                    connect_failed(std::move(error));
                    return false;
                }
                connect_failed(ErrorKind::protocol_violation, "expected +OK in reply to INIT, got '" + text.substr(0, 64) + "'");
                return false;
            case Phase::awaiting_info:
                if (frame.kind != Kind::info) {
                    connect_failed(ErrorKind::protocol_violation, "expected INFO after INIT +OK, got '" + text.substr(0, 64) + "'");
                    return false;
                }
                if (auto problem = owp::check_info(frame.payload, config_.owp_versions)) {
                    connect_failed(ErrorKind::protocol_violation, *problem);
                    return false;
                }
                info_ = frame.payload;
                phase_ = Phase::active;
                io_->connect_timer.cancel();
                {
                    std::lock_guard<std::mutex> lock(mutex_);
                    state_ = State::active;
                    connect_finished_ = true;
                    cv_.notify_all();
                }
                return true;
            case Phase::active:
                return handle_active_frame(frame);
            case Phase::connecting:
            case Phase::ended:
                return false;
        }
        return false;
    }

    bool handle_active_frame(owp::ServerFrame& frame) {
        using Kind = owp::ServerFrame::Kind;
        std::unique_lock<std::mutex> lock(mutex_);
        switch (frame.kind) {
            case Kind::ok:
            case Kind::err: {
                const bool ok = frame.kind == Kind::ok;
                if (pending_ && !pending_->done) {
                    std::shared_ptr<Pending> pending = std::move(pending_);
                    pending->done = true;
                    if (!ok) {
                        pending->error = make_error(ErrorKind::broker_rejected,
                                                    "-ERR " + frame.code + (frame.details.empty() ? "" : " " + frame.details));
                        pending->error.owp_code = frame.code;
                        pending->error.owp_details = frame.details;
                    }
                    if (pending->on_ack) {
                        pending->on_ack(ok);  // internal bookkeeping only (SUB activation)
                    }
                    cv_.notify_all();
                    return true;
                }
                if (ok) {
                    lock.unlock();
                    protocol_failure("+OK without an outstanding command");
                    return false;
                }
                Event event;
                event.kind = EventKind::server_error;
                event.error = make_error(ErrorKind::broker_rejected,
                                         "-ERR " + frame.code + (frame.details.empty() ? "" : " " + frame.details));
                event.error.owp_code = frame.code;
                event.error.owp_details = frame.details;
                push_event_locked(std::move(event));
                return true;
            }
            case Kind::info: {
                Event event;
                event.kind = EventKind::info;
                event.payload = std::move(frame.payload);
                push_event_locked(std::move(event));
                return true;
            }
            case Kind::msg: {
                if (inbound_.size() >= config_.max_pending_deliveries) {
                    lock.unlock();
                    fail_active(make_error(ErrorKind::delivery_overflow,
                                           "MSG backlog reached max_pending_deliveries (" +
                                               std::to_string(config_.max_pending_deliveries) +
                                               "); connection closed instead of dropping messages; the MSG for '" +
                                               frame.sid + "' that overflowed was not delivered"),
                                websocket::close_code::policy_error);
                    return false;
                }
                inbound_.push_back(Inbound{std::move(frame.sid), std::move(frame.payload)});
                lock.unlock();
                dispatch_cv_.notify_one();
                return true;
            }
            case Kind::invalid: break;
        }
        return true;
    }

    void protocol_failure(std::string message) {
        if (phase_ != Phase::active) {
            return connect_failed(ErrorKind::protocol_violation, std::move(message));
        }
        fail_active(make_error(ErrorKind::protocol_violation, std::move(message)), websocket::close_code::protocol_error);
    }

    void transport_lost(std::string message) {
        if (phase_ != Phase::active) {
            return connect_failed(ErrorKind::connect_failed, std::move(message));
        }
        phase_ = Phase::ended;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            end_locked(make_error(ErrorKind::connection_closed, std::move(message)));
        }
        beast::get_lowest_layer(io_->ws).close();
    }

    void fail_active(Error reason, websocket::close_code code) {
        {
            std::lock_guard<std::mutex> lock(mutex_);
            end_locked(reason);
        }
        abort_transport(code);
    }

    /// Ends the transport from the I/O thread: a bounded close handshake when
    /// the WebSocket is open, then the TCP socket is closed.
    void abort_transport(websocket::close_code code) {
        if (phase_ == Phase::ended && closing_started_) {
            return;
        }
        phase_ = Phase::ended;
        if (closing_started_) {
            return;
        }
        closing_started_ = true;
        write_queue_.clear();
        if (!io_->ws.is_open()) {
            beast::get_lowest_layer(io_->ws).close();
            return;
        }
        io_->close_timer.expires_after(config_.close_timeout);
        io_->close_timer.async_wait([this](beast::error_code ec) {
            if (!ec) {
                beast::get_lowest_layer(io_->ws).close();
            }
        });
        io_->ws.async_close(code, [this](beast::error_code) {
            io_->close_timer.cancel();
            beast::get_lowest_layer(io_->ws).close();
        });
    }

    void begin_close() {
        io_->connect_timer.cancel();
        io_->resolver.cancel();
        abort_transport(websocket::close_code::normal);
    }

    // ---- dispatcher thread -----------------------------------------------------

    void dispatch_main() {
        std::unique_lock<std::mutex> lock(mutex_);
        for (;;) {
            dispatch_cv_.wait(lock, [this] { return stop_dispatch_ || !inbound_.empty(); });
            if (stop_dispatch_) {
                stats_.discarded_on_close += inbound_.size();
                inbound_.clear();
                return;
            }
            Inbound message = std::move(inbound_.front());
            inbound_.pop_front();
            auto it = subs_.find(message.sid);
            if (it == subs_.end() || !it->second->active) {
                ++stats_.unknown_subscription;
                Event event;
                event.kind = EventKind::unknown_subscription;
                event.sid = std::move(message.sid);
                event.payload = std::move(message.payload);
                push_event_locked(std::move(event));
                continue;
            }
            std::shared_ptr<SubEntry> entry = it->second;
            entry->running = true;
            lock.unlock();

            Error error;
            bool delivered = false;
            bool threw = false;
            const owp::Unwrapped unwrapped = owp::unwrap_global_element(message.payload, entry->element);
            if (!unwrapped.ok) {
                error = make_error(ErrorKind::codec, unwrapped.problem);
            } else {
                try {
                    delivered = entry->delivery->deliver(unwrapped.body, error);
                } catch (const std::exception& exception) {
                    threw = true;
                    error = make_error(ErrorKind::none, std::string("handler or codec threw: ") + exception.what());
                } catch (...) {
                    threw = true;
                    error = make_error(ErrorKind::none, "handler or codec threw a non-standard exception");
                }
            }

            lock.lock();
            entry->running = false;
            sub_cv_.notify_all();
            Event event;
            event.sid = entry->sid;
            event.message_name = entry->element;
            if (threw) {
                ++stats_.handler_exceptions;
                event.kind = EventKind::handler_exception;
                event.error = std::move(error);
                push_event_locked(std::move(event));
            } else if (!delivered) {
                ++stats_.decode_errors;
                event.kind = EventKind::decode_error;
                event.error = std::move(error);
                event.payload = std::move(message.payload);
                push_event_locked(std::move(event));
            } else {
                ++stats_.delivered;
            }
            // Release this thread's reference without the lock: if the
            // subscription was removed meanwhile, this destroys its handler.
            lock.unlock();
            entry.reset();
            lock.lock();
        }
    }

    // ---- data ------------------------------------------------------------------

    const Config config_;
    const Endpoint endpoint_;

    // I/O-thread-owned. Bundled so close() can destroy every Asio object --
    // and so join Asio's internal resolver thread -- deterministically.
    struct Io {
        net::io_context ioc;
        net::executor_work_guard<net::io_context::executor_type> work{net::make_work_guard(ioc)};
        tcp::resolver resolver{ioc};
        websocket::stream<beast::tcp_stream> ws{ioc};
        net::steady_timer connect_timer{ioc};
        net::steady_timer close_timer{ioc};
    };
    std::unique_ptr<Io> io_;  // reset only by close(), after the I/O thread joined
    websocket::response_type handshake_response_;
    beast::flat_buffer read_buffer_;
    std::deque<std::string> write_queue_;
    bool writing_ = false;
    bool closing_started_ = false;
    bool connect_timed_out_ = false;
    Phase phase_ = Phase::connecting;
    std::string info_;

    std::thread io_thread_;
    std::thread dispatch_thread_;
    std::atomic<std::thread::id> io_id_{};
    std::atomic<std::thread::id> dispatch_id_{};

    // shared, guarded by mutex_
    mutable std::mutex mutex_;
    std::condition_variable cv_;           // connect / command completion / io exit
    std::condition_variable dispatch_cv_;  // inbound backlog
    std::condition_variable event_cv_;     // events
    std::condition_variable sub_cv_;       // entry->running changes
    std::condition_variable slot_cv_;      // command_busy_ changes
    bool command_busy_ = false;            // one acknowledged command at a time
    State state_ = State::connecting;
    Error failure_;
    bool connect_finished_ = false;
    Error connect_error_;
    bool io_exited_ = false;
    bool stop_dispatch_ = false;
    bool joined_ = false;
    std::shared_ptr<Pending> pending_;
    std::uint64_t next_sid_ = 1;
    std::map<std::string, std::shared_ptr<SubEntry>> subs_;
    std::deque<Inbound> inbound_;
    std::deque<Event> events_;
    std::uint64_t dropped_unreported_ = 0;
    Stats stats_;

    std::mutex close_mutex_;
};

}  // namespace detail

// ---- Runtime -------------------------------------------------------------------

Result<Runtime> Runtime::connect(Config config) {
    using detail::make_error;
    if (!owp::is_identifier(config.service_id)) {
        return Result<Runtime>::failure(make_error(ErrorKind::invalid_argument,
                                                   "service_id '" + config.service_id + "' is not an OWP identifier"));
    }
    if (config.schema_version.empty()) {
        return Result<Runtime>::failure(make_error(ErrorKind::invalid_argument, "schema_version is required"));
    }
    for (const char c : config.schema_version) {
        if (static_cast<unsigned char>(c) < 0x20 || c == 0x7F) {
            return Result<Runtime>::failure(make_error(ErrorKind::invalid_argument, "schema_version contains a control character"));
        }
    }
    if (config.owp_versions.empty()) {
        return Result<Runtime>::failure(make_error(ErrorKind::invalid_argument, "owp_versions is empty"));
    }
    for (const std::string& version : config.owp_versions) {
        if (version != owp::owp_version_1_0) {
            return Result<Runtime>::failure(make_error(ErrorKind::invalid_argument,
                                                       "OWP version '" + version + "' is not implemented (only 1.0)"));
        }
    }
    using std::chrono::milliseconds;
    if (config.connect_timeout <= milliseconds(0) || config.operation_timeout <= milliseconds(0) ||
        config.close_timeout <= milliseconds(0)) {
        return Result<Runtime>::failure(make_error(ErrorKind::invalid_argument, "timeouts must be positive"));
    }
    if (config.max_message_bytes == 0 || config.max_pending_deliveries == 0 || config.max_pending_events == 0) {
        return Result<Runtime>::failure(make_error(ErrorKind::invalid_argument, "bounds must be positive"));
    }
    auto endpoint = detail::parse_url(config.url);
    if (!endpoint.ok()) {
        return Result<Runtime>::failure(endpoint.error());
    }
    auto core = std::make_shared<detail::Core>(std::move(config), std::move(endpoint).value());
    Status started = core->start();
    if (!started.ok()) {
        (void)core->close();
        return Result<Runtime>::failure(started.error());
    }
    return Result<Runtime>::success(Runtime(std::move(core)));
}

Runtime& Runtime::operator=(Runtime&& other) noexcept {
    if (this != &other) {
        if (core_) {
            (void)close();
        }
        core_ = std::move(other.core_);
    }
    return *this;
}

Runtime::~Runtime() {
    if (!core_) {
        return;
    }
    if (core_->on_runtime_thread()) {
        std::fputs("ams_gra::lacal: a Runtime was destroyed from one of its own handlers\n", stderr);
        std::terminate();
    }
    (void)core_->close();
}

Status Runtime::close() {
    if (!core_) {
        return Status::success();
    }
    return core_->close();
}

std::optional<Event> Runtime::poll_event(std::chrono::milliseconds timeout) {
    if (!core_) {
        return std::nullopt;
    }
    return core_->poll_event(timeout);
}

bool Runtime::connected() const { return core_ && core_->connected(); }

Stats Runtime::stats() const { return core_ ? core_->stats() : Stats{}; }

const std::string& Runtime::server_info() const {
    static const std::string empty;
    return core_ ? core_->server_info() : empty;
}

Status Runtime::publish_body(std::string element, std::string_view topic, const std::string& body) {
    if (!core_) {
        return Status::failure(detail::make_error(ErrorKind::connection_closed, "moved-from Runtime"));
    }
    return core_->publish(std::move(element), topic, body);
}

Result<Subscription> Runtime::subscribe_delivery(std::string element, std::string_view topic,
                                                 std::optional<std::string_view> group,
                                                 std::unique_ptr<detail::Delivery> delivery) {
    if (!core_) {
        return Result<Subscription>::failure(detail::make_error(ErrorKind::connection_closed, "moved-from Runtime"));
    }
    auto sid = core_->subscribe(std::move(element), topic, group, std::move(delivery));
    if (!sid.ok()) {
        return Result<Subscription>::failure(sid.error());
    }
    return Result<Subscription>::success(Subscription(core_, std::move(sid).value()));
}

// ---- Subscription --------------------------------------------------------------

Subscription::Subscription(Subscription&& other) noexcept
    : core_(std::move(other.core_)), sid_(std::move(other.sid_)), active_(other.active_) {
    other.active_ = false;
}

Subscription& Subscription::operator=(Subscription&& other) noexcept {
    if (this != &other) {
        if (active_) {
            (void)unsubscribe();
        }
        core_ = std::move(other.core_);
        sid_ = std::move(other.sid_);
        active_ = other.active_;
        other.active_ = false;
    }
    return *this;
}

Subscription::~Subscription() {
    if (active_) {
        (void)unsubscribe();
    }
}

Status Subscription::unsubscribe() {
    if (!active_) {
        return Status::failure(detail::make_error(ErrorKind::subscription_inactive, "subscription handle is not active"));
    }
    active_ = false;
    std::shared_ptr<detail::Core> core = core_.lock();
    if (!core) {
        return Status::failure(detail::make_error(ErrorKind::connection_closed, "runtime is closed"));
    }
    return core->unsubscribe(sid_);
}

}  // namespace ams_gra::lacal
