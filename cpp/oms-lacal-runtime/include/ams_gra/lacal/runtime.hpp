// CPP-OMS-003: reusable C++17 LA-CAL runtime over OWP 1.0 / WebSocket.
//
// `Runtime` is the adapter the generated `service_api.hpp` facade forwards to:
//
//   auto runtime = ams_gra::lacal::Runtime::connect(config);          // Result<Runtime>
//   service_api::function_log::exchange_publish::publish(*runtime, value);   // Status
//   auto sub = service_api::function_log::exchange_receive::subscribe(
//       *runtime, [](const Payload& p) { ... });                     // Result<Subscription>
//
// Payload bodies are encoded and decoded ONLY by the generated
// `service_codec.hpp` functions, bound once per payload type with
// AMS_GRA_LACAL_PAYLOAD_CODEC (no handwritten wire codec). The runtime owns
// the global-element envelope, OWP framing, subscription IDs, dispatch and
// the connection. See docs/task-cpp-oms-003-lacal-runtime.md.
//
// Threads: each connected Runtime owns exactly two threads, joined by
// close() (or the destructor): an I/O thread that owns the WebSocket and
// every socket operation, and a dispatcher thread that decodes payloads and
// runs handlers. Handlers never run with a runtime lock held, so they may
// publish, subscribe and unsubscribe; they may not close/destroy the Runtime.
//
// OWP is used in verbose mode with one outstanding command: publish(),
// subscribe() and unsubscribe() return only after the peer's +OK or -ERR for
// that command (or a bounded timeout). Success therefore means the peer
// acknowledged the operation, not merely that a frame was written.
#pragma once

#include <chrono>
#include <cstddef>
#include <cstdint>
#include <exception>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <vector>

#include "ams_gra/lacal/owp.hpp"

namespace ams_gra::lacal {

// ---- errors ----------------------------------------------------------------

enum class ErrorKind {
    none,
    invalid_argument,            // rejected locally before any frame (config, identifier, QName)
    unsupported_scheme,          // not ws:// (wss:// is NOT implemented and is refused)
    codec,                       // generated encoder rejected the value; nothing was sent
    connect_failed,              // DNS / TCP / WebSocket transport failure while connecting
    handshake_rejected,          // HTTP upgrade declined, or subprotocol is not exactly "owp"
    broker_rejected,             // the peer answered this operation with -ERR
    protocol_violation,          // the peer sent something that is not valid OWP 1.0
    timeout,                     // a bounded wait expired
    connection_closed,           // not connected, connection lost, or Runtime closed
    subscription_inactive,       // unsubscribe on a handle that is no longer active
    subscription_ids_exhausted,  // the 64-bit SID counter would wrap
    delivery_overflow,           // undelivered MSG backlog exceeded its bound
    called_from_runtime_thread,  // operation not allowed from a runtime-owned thread
};

const char* to_string(ErrorKind kind) noexcept;

struct Error {
    ErrorKind kind = ErrorKind::none;
    std::string message;
    std::string owp_code;    // broker_rejected: e.g. "Illegal-State"
    std::string owp_details; // broker_rejected: the -ERR details text
    std::string codec_path;  // codec: schema-derived path from the generated codec
};

/// Success or an Error. Never throws.
class [[nodiscard]] Status {
public:
    Status() = default;
    static Status success() { return Status(); }
    static Status failure(Error error) {
        Status status;
        status.error_ = std::move(error);
        return status;
    }
    bool ok() const noexcept { return error_.kind == ErrorKind::none; }
    explicit operator bool() const noexcept { return ok(); }
    const Error& error() const noexcept { return error_; }

private:
    Error error_;
};

/// A value or an Error. `value()` / `operator*` require `ok()`.
template <typename T>
class [[nodiscard]] Result {
public:
    static Result success(T value) {
        Result result;
        result.value_.emplace(std::move(value));
        return result;
    }
    static Result failure(Error error) {
        Result result;
        result.error_ = std::move(error);
        return result;
    }
    bool ok() const noexcept { return value_.has_value(); }
    explicit operator bool() const noexcept { return ok(); }
    T& value() & noexcept { return *value_; }
    const T& value() const& noexcept { return *value_; }
    T&& value() && noexcept { return std::move(*value_); }
    T& operator*() & noexcept { return *value_; }
    T* operator->() noexcept { return &*value_; }
    const Error& error() const noexcept { return error_; }

private:
    Result() = default;
    std::optional<T> value_;
    Error error_;
};

// ---- configuration ---------------------------------------------------------

struct Config {
    std::string url;             // ws://host[:port][/path]; wss:// is refused
    std::string service_id;      // OWP identifier
    std::string schema_version;  // e.g. "002.5.0"; required, never defaulted
    std::vector<std::string> owp_versions{std::string(owp::owp_version_1_0)};
    std::chrono::milliseconds connect_timeout{10000};    // TCP + upgrade + INIT/+OK/INFO
    std::chrono::milliseconds operation_timeout{10000};  // each acknowledged command
    std::chrono::milliseconds close_timeout{2000};       // WebSocket close handshake
    std::size_t max_message_bytes = 16U * 1024U * 1024U; // largest accepted incoming frame
    std::size_t max_pending_deliveries = 4096;           // undelivered MSG backlog bound
    std::size_t max_pending_events = 1024;               // unread event bound (then counted)
};

// ---- asynchronous events -----------------------------------------------------

enum class EventKind {
    server_error,          // -ERR not paired with any outstanding command
    decode_error,          // envelope or generated decoder rejected a MSG (subscription stays active)
    unknown_subscription,  // MSG for a SID with no active subscription (no handler ran)
    handler_exception,     // a handler threw (contained; subscription stays active)
    info,                  // INFO received after initialization
    connection_closed,     // the connection ended (reason in `error`)
    events_dropped,        // `count` events were not queued because the event queue was full
};

const char* to_string(EventKind kind) noexcept;

struct Event {
    EventKind kind = EventKind::server_error;
    std::string sid;           // decode_error / unknown_subscription / handler_exception
    std::string message_name;  // decode_error: the subscribed global element name
    std::string payload;       // unknown_subscription / decode_error / info: raw text
    Error error;               // server_error, decode_error, connection_closed, handler_exception
    std::uint64_t count = 0;   // events_dropped
};

struct Stats {
    std::uint64_t delivered = 0;             // handler invocations that returned normally
    std::uint64_t decode_errors = 0;
    std::uint64_t unknown_subscription = 0;
    std::uint64_t handler_exceptions = 0;
    std::uint64_t discarded_on_close = 0;    // queued MSGs not dispatched because close() ran
    std::uint64_t events_dropped = 0;
};

// ---- payload codec binding (customisation point) ---------------------------

/// Specialise (normally with AMS_GRA_LACAL_PAYLOAD_CODEC) to bind a generated
/// payload type to its generated `service_codec::encode_P` / `decode_P`.
template <typename P>
struct payload_codec {
    static constexpr bool bound = false;
};

namespace detail {
class Core;

/// Type-erased typed delivery: decodes one payload body and invokes the
/// handler. Returns false and fills `error` when the codec rejected it.
class Delivery {
public:
    virtual ~Delivery() = default;
    virtual bool deliver(std::string_view body, Error& error) = 0;
};

template <typename P, typename H>
class TypedDelivery final : public Delivery {
public:
    explicit TypedDelivery(H handler) : handler_(std::move(handler)) {}
    bool deliver(std::string_view body, Error& error) override {
        auto decoded = payload_codec<P>::decode(body);
        if (!decoded.ok()) {
            error.kind = ErrorKind::codec;
            error.codec_path = decoded.error().path;
            error.message = decoded.error().message;
            return false;
        }
        const P& value = decoded.value();
        handler_(value);
        return true;
    }

private:
    H handler_;
};
}  // namespace detail

// ---- subscriptions -----------------------------------------------------------

/// Owns one active subscription. Move-only. Destroying (or move-assigning
/// over) an active handle unsubscribes it exactly as unsubscribe() does,
/// bounded by operation_timeout; call unsubscribe() to observe the outcome.
/// After unsubscribe() returns, the handler is never invoked again and is not
/// running (unless unsubscribe() was called from that handler itself).
class [[nodiscard]] Subscription {
public:
    Subscription() = default;
    Subscription(Subscription&& other) noexcept;
    Subscription& operator=(Subscription&& other) noexcept;
    Subscription(const Subscription&) = delete;
    Subscription& operator=(const Subscription&) = delete;
    ~Subscription();

    const std::string& id() const noexcept { return sid_; }
    bool active() const noexcept { return active_; }
    /// Removes local dispatch first, then sends UNSUB and waits for +OK/-ERR.
    /// One-shot: the handle is inactive afterwards whatever the outcome.
    Status unsubscribe();

private:
    friend class Runtime;
    Subscription(std::weak_ptr<detail::Core> core, std::string sid)
        : core_(std::move(core)), sid_(std::move(sid)), active_(true) {}
    std::weak_ptr<detail::Core> core_;
    std::string sid_;
    bool active_ = false;
};

// ---- runtime -----------------------------------------------------------------

class Runtime {
public:
    /// Connects, upgrades with subprotocol `owp`, sends INIT (verbose) and
    /// requires +OK then a valid INFO, all within connect_timeout.
    static Result<Runtime> connect(Config config);

    Runtime(Runtime&& other) noexcept = default;
    Runtime& operator=(Runtime&& other) noexcept;
    Runtime(const Runtime&) = delete;
    Runtime& operator=(const Runtime&) = delete;
    /// Calls close(). Destroying a Runtime from one of its own handlers is a
    /// contract violation and terminates the process (it cannot join itself).
    ~Runtime();

    /// Generated-facade publish hook. Encodes with the bound generated codec
    /// (codec errors send nothing), wraps `{"<element>": body}` and sends PUB.
    template <typename P>
    Status publish(std::string_view message_namespace, std::string_view message_name,
                   std::string_view topic, const P& value) {
        static_assert(payload_codec<P>::bound,
                      "bind this payload type with AMS_GRA_LACAL_PAYLOAD_CODEC(Type, encode_fn, decode_fn)");
        if (auto problem = owp::check_message_qname(message_namespace, message_name)) {
            return Status::failure(Error{ErrorKind::invalid_argument, std::move(*problem), {}, {}, {}});
        }
        auto encoded = payload_codec<P>::encode(value);
        if (!encoded.ok()) {
            Error error{ErrorKind::codec, encoded.error().message, {}, {}, encoded.error().path};
            return Status::failure(std::move(error));
        }
        return publish_body(owp::global_element_name(message_namespace, message_name), topic,
                            encoded.value());
    }

    /// Generated-facade subscribe hook. Allocates a connection-local SID
    /// (`sub-N`), sends SUB and activates dispatch only on the peer's +OK.
    template <typename P, typename H>
    Result<Subscription> subscribe(std::string_view message_namespace, std::string_view message_name,
                                   std::string_view topic, std::optional<std::string_view> group,
                                   H&& handler) {
        static_assert(payload_codec<P>::bound,
                      "bind this payload type with AMS_GRA_LACAL_PAYLOAD_CODEC(Type, encode_fn, decode_fn)");
        using Handler = std::decay_t<H>;
        static_assert(std::is_invocable_v<Handler&, const P&>, "handler must accept const P&");
        if (auto problem = owp::check_message_qname(message_namespace, message_name)) {
            return Result<Subscription>::failure(
                Error{ErrorKind::invalid_argument, std::move(*problem), {}, {}, {}});
        }
        std::unique_ptr<detail::Delivery> delivery =
            std::make_unique<detail::TypedDelivery<P, Handler>>(Handler(std::forward<H>(handler)));
        return subscribe_delivery(owp::global_element_name(message_namespace, message_name), topic,
                                  group, std::move(delivery));
    }

    /// Next asynchronous event, waiting at most `timeout`.
    std::optional<Event> poll_event(std::chrono::milliseconds timeout = std::chrono::milliseconds(0));

    /// Ends the connection (WebSocket close, bounded by close_timeout), stops
    /// dispatch after the running handler returns, and joins both threads.
    /// Idempotent. Refused (called_from_runtime_thread) inside a handler.
    Status close();

    bool connected() const;
    Stats stats() const;
    /// The validated INFO payload received during initialization.
    const std::string& server_info() const;

private:
    explicit Runtime(std::shared_ptr<detail::Core> core) : core_(std::move(core)) {}
    Status publish_body(std::string element, std::string_view topic, const std::string& body);
    Result<Subscription> subscribe_delivery(std::string element, std::string_view topic,
                                            std::optional<std::string_view> group,
                                            std::unique_ptr<detail::Delivery> delivery);
    std::shared_ptr<detail::Core> core_;
};

}  // namespace ams_gra::lacal

/// Binds generated payload type `PayloadType` to the generated codec
/// functions. Use once per payload type at global namespace scope, e.g.
///   AMS_GRA_LACAL_PAYLOAD_CODEC(::programs::oam::LogMT,
///                               ::service_codec::encode_LogMT,
///                               ::service_codec::decode_LogMT)
#define AMS_GRA_LACAL_PAYLOAD_CODEC(PayloadType, EncodeFunction, DecodeFunction) \
    namespace ams_gra::lacal {                                                  \
    template <>                                                                 \
    struct payload_codec<PayloadType> {                                         \
        static constexpr bool bound = true;                                     \
        static auto encode(const PayloadType& value) { return EncodeFunction(value); } \
        static auto decode(std::string_view json) { return DecodeFunction(json); }     \
    };                                                                          \
    }
