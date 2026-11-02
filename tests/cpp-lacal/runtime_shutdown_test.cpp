// CPP-OMS-003R1: asynchronous write ownership during shutdown, and memory
// bounds, driven through the GENERATED service API (runtime-test.yaml,
// namespace urn:test) against the test-only controlled peer.
//
// Write-ownership cases are deterministic, not timing-based:
//  * the peer stops reading after the frames each case needs and advertises
//    a small receive window, and the outgoing PUB is larger than every buffer
//    between the two sockets (sized from the kernel's own limits), so the
//    runtime's async_write cannot complete until the peer resumes reading;
//  * detail::TestAccess::io_snapshot runs on the runtime's I/O thread and
//    reports whether that write is still outstanding and whether the close
//    handshake was deferred behind it, so each assertion is placed at an
//    exact state of the I/O state machine rather than after a sleep.
// Every case is run on both completion paths of the pending write: an error
// completion (the bounded close or the peer cuts the connection) and a
// successful completion (the peer drains the frame, which must arrive intact).
//
// Build: see scripts/check-cpp-oms-003-runtime.sh.
#include <algorithm>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <fstream>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <vector>

#include "ams_gra/lacal/runtime.hpp"
#include "check.hpp"
#include "mock_peer.hpp"
#include "service_api.hpp"
#include "service_codec.hpp"
#include "test.hpp"

AMS_GRA_LACAL_PAYLOAD_CODEC(::urn::test::SharedPayload, ::service_codec::encode_SharedPayload,
                            ::service_codec::decode_SharedPayload)
AMS_GRA_LACAL_PAYLOAD_CODEC(::urn::test::OtherPayload, ::service_codec::encode_OtherPayload,
                            ::service_codec::decode_OtherPayload)

namespace lacal = ams_gra::lacal;
namespace track = service_api::function_track;
using lacal::ErrorKind;
using lacal::EventKind;
using lacal::detail::IoSnapshot;
using lacal::detail::TestAccess;
using std::chrono::milliseconds;
using Clock = std::chrono::steady_clock;

namespace {

constexpr int kPeerReceiveBuffer = 64 * 1024;  // the kernel doubles it

lacal::Config config_for(const std::string& url) {
    lacal::Config config;
    config.url = url;
    config.service_id = "svc-1";
    config.schema_version = "000.1.0";
    config.connect_timeout = milliseconds(5000);
    config.operation_timeout = milliseconds(20000);
    config.close_timeout = milliseconds(20000);
    return config;
}

lacal::Runtime connect_ok(const lacal::Config& config) {
    auto runtime = lacal::Runtime::connect(config);
    if (!runtime.ok()) {
        std::fprintf(stderr, "test setup: connect failed: %s: %s\n", lacal::to_string(runtime.error().kind),
                     runtime.error().message.c_str());
        std::exit(2);
    }
    return std::move(runtime).value();
}

/// Largest value in a /proc/sys file of whitespace-separated integers (0 if unreadable).
std::size_t proc_max(const char* path) {
    std::ifstream in(path);
    std::size_t best = 0;
    std::size_t value = 0;
    while (in >> value) {
        best = std::max(best, value);
    }
    return best;
}

/// A PUB frame larger than everything that can buffer it between the
/// runtime's socket and the paused peer: the sender's autotuned send buffer
/// (bounded by tcp_wmem / wmem_max) plus the peer's fixed receive window.
std::size_t blocking_label_size() {
    const std::size_t send_buffer = std::max(proc_max("/proc/sys/net/ipv4/tcp_wmem"),
                                             proc_max("/proc/sys/net/core/wmem_max"));
    const std::size_t capacity = send_buffer + 4U * static_cast<std::size_t>(kPeerReceiveBuffer);
    return std::max<std::size_t>(16U << 20U, 4U * capacity);
}

::urn::test::OtherPayload big_payload() {
    return ::urn::test::OtherPayload{std::string(blocking_label_size(), 'w')};
}

/// The exact PUB text the runtime must put on the wire for `value`.
std::string expected_pub(const ::urn::test::OtherPayload& value) {
    auto encoded = ::service_codec::encode_OtherPayload(value);
    if (!encoded.ok()) {
        std::exit(2);
    }
    return lacal::owp::build_pub("output-topic", "{urn:test}MessageC", encoded.value());
}

::urn::test::SharedPayload shared(std::int64_t count) {
    auto value = decltype(::urn::test::SharedPayload::count)::create(count);
    if (!value) {
        std::exit(2);
    }
    return ::urn::test::SharedPayload{*value};
}

std::string msg(const std::string& sid, std::int64_t count) {
    return "MSG " + sid + " {\"{urn:test}MessageA\":{\"{urn:test}Count\":" + std::to_string(count) + "}}";
}

/// Polls the I/O thread until `predicate(snapshot)` holds.
template <typename Predicate>
std::optional<IoSnapshot> io_until(lacal::Runtime& runtime, Predicate predicate,
                                   milliseconds timeout = milliseconds(10000)) {
    std::optional<IoSnapshot> last;
    const bool reached = check::eventually(
        [&] {
            last = TestAccess::io_snapshot(runtime, milliseconds(2000));
            return last && predicate(*last);
        },
        timeout);
    return reached ? last : std::nullopt;
}

/// A publish of a frame too large to drain, run on its own application
/// thread; the result is read after join().
struct BlockedPublish {
    explicit BlockedPublish(lacal::Runtime& runtime, const ::urn::test::OtherPayload& value)
        : thread([this, &runtime, &value] { status = track::exchange_output_c::publish(runtime, value); }) {}
    void join() { thread.join(); }
    lacal::Status status;
    std::thread thread;
};

bool write_outstanding(const IoSnapshot& s) { return s.write_in_flight && !s.shutdown_started; }
bool close_deferred_behind_write(const IoSnapshot& s) {
    return s.shutdown_started && s.write_in_flight && s.close_deferred && !s.close_handshake_started &&
           !s.transport_closed && s.queued_frames == 0;
}
bool transport_finished(const IoSnapshot& s) { return s.transport_closed && !s.write_in_flight; }

/// The PUB is verifiably stuck: outstanding on the runtime side, with bytes
/// sitting unread in the peer's kernel buffer.
void require_write_stuck(lacal::Runtime& runtime, mock::Peer& peer, std::size_t frame_bytes) {
    auto pending = io_until(runtime, write_outstanding);
    CHECK(pending.has_value());
    CHECK(pending && pending->in_flight_bytes == frame_bytes);
    CHECK(check::eventually([&] { return peer.unread_bytes() > 0; }));
    auto still = TestAccess::io_snapshot(runtime, milliseconds(2000));
    CHECK(still && still->write_in_flight);
}

bool no_runtime_threads() {
    return check::eventually([] { return check::threads_named("ams-lacal") == 0; }, milliseconds(2000));
}

}  // namespace

// ---- application close while the write is outstanding ---------------------------

TEST_CASE(close_while_write_pending_error_completion) {
    mock::Peer peer(mock::acknowledge_everything, mock::Peer::Handshake::owp, kPeerReceiveBuffer);
    peer.pause_reading_after(1);  // INIT only
    lacal::Config config = config_for(peer.url());
    config.close_timeout = milliseconds(300);
    lacal::Runtime runtime = connect_ok(config);
    CHECK(peer.wait_paused());
    const auto value = big_payload();
    const std::string expected = expected_pub(value);
    BlockedPublish publish(runtime, value);
    require_write_stuck(runtime, peer, expected.size());

    const auto started = Clock::now();
    std::thread closer([&] { CHECK(runtime.close().ok()); });
    // close() discards nothing in flight: the close frame waits behind the write.
    CHECK(io_until(runtime, close_deferred_behind_write).has_value());
    closer.join();
    publish.join();
    // The peer never drains: close_timeout closes the socket, the write
    // completes with an error, and close() returns within its bound.
    CHECK(Clock::now() - started < config.close_timeout + milliseconds(2500));
    CHECK(publish.status.error().kind == ErrorKind::connection_closed);  // never a fabricated +OK
    const lacal::Stats stats = runtime.stats();
    CHECK_EQ(stats.writes_aborted, 1U);
    CHECK_EQ(stats.unsent_frames_discarded, 0U);
    CHECK(!peer.client_sent_close());  // no close frame was interleaved into the pending frame
    CHECK_EQ(peer.frames().size(), 1U);
    CHECK(no_runtime_threads());
}

TEST_CASE(close_while_write_pending_success_completion) {
    mock::Peer peer(mock::acknowledge_everything, mock::Peer::Handshake::owp, kPeerReceiveBuffer);
    peer.pause_reading_after(1);
    lacal::Runtime runtime = connect_ok(config_for(peer.url()));  // close_timeout 20 s: never reached
    CHECK(peer.wait_paused());
    const auto value = big_payload();
    const std::string expected = expected_pub(value);
    BlockedPublish publish(runtime, value);
    require_write_stuck(runtime, peer, expected.size());

    const auto started = Clock::now();
    std::thread closer([&] { CHECK(runtime.close().ok()); });
    CHECK(io_until(runtime, close_deferred_behind_write).has_value());
    peer.resume_reading();
    closer.join();
    publish.join();
    CHECK(Clock::now() - started < milliseconds(10000));  // the write finished; no timer expiry
    CHECK(publish.status.error().kind == ErrorKind::connection_closed);
    CHECK(peer.wait_client_close());
    CHECK_EQ(peer.client_close_code(), 1000U);  // normal: the deferred close handshake ran
    const auto frames = peer.frames();
    CHECK_EQ(frames.size(), 2U);
    // The frame Beast was writing when shutdown began arrived byte-for-byte.
    CHECK(frames.size() == 2 && frames[1].size() == expected.size());
    CHECK(frames.size() == 2 && frames[1] == expected);
    CHECK_EQ(runtime.stats().writes_aborted, 0U);
    CHECK(no_runtime_threads());
}

// ---- command timeout while the write is outstanding -----------------------------

TEST_CASE(operation_timeout_while_write_pending_error_completion) {
    mock::Peer peer(mock::acknowledge_everything, mock::Peer::Handshake::owp, kPeerReceiveBuffer);
    peer.pause_reading_after(1);
    lacal::Config config = config_for(peer.url());
    config.operation_timeout = milliseconds(400);
    config.close_timeout = milliseconds(300);
    lacal::Runtime runtime = connect_ok(config);
    CHECK(peer.wait_paused());
    const auto value = big_payload();
    lacal::Status status = track::exchange_output_c::publish(runtime, value);
    CHECK(status.error().kind == ErrorKind::timeout);
    // The shutdown was queued on the I/O thread before publish() returned,
    // so the first snapshot already sees the deferred close.
    auto snapshot = TestAccess::io_snapshot(runtime, milliseconds(2000));
    CHECK(snapshot && close_deferred_behind_write(*snapshot));
    CHECK(io_until(runtime, transport_finished).has_value());
    CHECK_EQ(runtime.stats().writes_aborted, 1U);
    auto event = runtime.poll_event(milliseconds(2000));
    CHECK(event && event->kind == EventKind::connection_closed && event->error.kind == ErrorKind::timeout);
    CHECK(!runtime.connected());
    CHECK(track::exchange_output_a::publish(runtime, shared(1)).error().kind == ErrorKind::connection_closed);
    CHECK(!peer.client_sent_close());
    CHECK(runtime.close().ok());
    CHECK(no_runtime_threads());
}

TEST_CASE(operation_timeout_while_write_pending_success_completion) {
    mock::Peer peer(mock::acknowledge_everything, mock::Peer::Handshake::owp, kPeerReceiveBuffer);
    peer.pause_reading_after(1);
    lacal::Config config = config_for(peer.url());
    config.operation_timeout = milliseconds(400);
    lacal::Runtime runtime = connect_ok(config);
    CHECK(peer.wait_paused());
    const auto value = big_payload();
    const std::string expected = expected_pub(value);
    lacal::Status status = track::exchange_output_c::publish(runtime, value);
    CHECK(status.error().kind == ErrorKind::timeout);
    auto snapshot = TestAccess::io_snapshot(runtime, milliseconds(2000));
    CHECK(snapshot && close_deferred_behind_write(*snapshot));
    peer.resume_reading();
    CHECK(peer.wait_client_close(milliseconds(10000)));
    CHECK_EQ(peer.client_close_code(), 1008U);  // policy_error: ended because of the timeout
    const auto frames = peer.frames();
    CHECK(frames.size() >= 2 && frames[1] == expected);
    CHECK(io_until(runtime, transport_finished).has_value());
    CHECK_EQ(runtime.stats().writes_aborted, 0U);
    // The peer's late +OK for that PUB is never paired with anything.
    CHECK(track::exchange_output_a::publish(runtime, shared(2)).error().kind == ErrorKind::connection_closed);
    CHECK(runtime.close().ok());
    CHECK(no_runtime_threads());
}

// ---- connection loss, protocol failure and overflow while the write is pending ---

TEST_CASE(connection_loss_while_write_pending) {
    mock::Peer peer(mock::acknowledge_everything, mock::Peer::Handshake::owp, kPeerReceiveBuffer);
    peer.pause_reading_after(1);
    lacal::Runtime runtime = connect_ok(config_for(peer.url()));
    CHECK(peer.wait_paused());
    const auto value = big_payload();
    BlockedPublish publish(runtime, value);
    require_write_stuck(runtime, peer, expected_pub(value).size());
    peer.inject({mock::Action::drop()});
    publish.join();
    CHECK(publish.status.error().kind == ErrorKind::connection_closed);
    auto finished = io_until(runtime, transport_finished);
    CHECK(finished && finished->shutdown_started && !finished->close_handshake_started);
    CHECK_EQ(runtime.stats().writes_aborted, 1U);
    auto event = runtime.poll_event(milliseconds(2000));
    CHECK(event && event->kind == EventKind::connection_closed);
    CHECK(runtime.close().ok());
    CHECK(no_runtime_threads());
}

TEST_CASE(protocol_failure_while_write_pending_defers_close) {
    mock::Peer peer(mock::acknowledge_everything, mock::Peer::Handshake::owp, kPeerReceiveBuffer);
    peer.pause_reading_after(1);
    lacal::Runtime runtime = connect_ok(config_for(peer.url()));
    CHECK(peer.wait_paused());
    const auto value = big_payload();
    const std::string expected = expected_pub(value);
    BlockedPublish publish(runtime, value);
    require_write_stuck(runtime, peer, expected.size());
    peer.inject({mock::Action::text("NOT-AN-OWP-OPERATION")});  // the peer can still send
    publish.join();
    CHECK(publish.status.error().kind == ErrorKind::protocol_violation);
    CHECK(io_until(runtime, close_deferred_behind_write).has_value());
    peer.resume_reading();
    CHECK(peer.wait_client_close(milliseconds(10000)));
    CHECK_EQ(peer.client_close_code(), 1002U);  // protocol_error
    const auto frames = peer.frames();
    CHECK(frames.size() >= 2 && frames[1] == expected);
    CHECK(io_until(runtime, transport_finished).has_value());
    CHECK_EQ(runtime.stats().writes_aborted, 0U);
    CHECK(runtime.close().ok());
    CHECK(no_runtime_threads());
}

TEST_CASE(delivery_overflow_while_write_pending) {
    mock::Peer peer(mock::acknowledge_everything, mock::Peer::Handshake::owp, kPeerReceiveBuffer);
    peer.pause_reading_after(2);  // INIT and SUB
    lacal::Config config = config_for(peer.url());
    config.max_pending_deliveries = 1;
    config.close_timeout = milliseconds(300);
    lacal::Runtime runtime = connect_ok(config);
    std::mutex mutex;
    std::condition_variable cv;
    bool entered = false;
    bool release = false;
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto&) {
        std::unique_lock<std::mutex> lock(mutex);
        entered = true;
        cv.notify_all();
        cv.wait(lock, [&] { return release; });
    });
    CHECK(subscription.ok());
    CHECK(peer.wait_paused());
    const auto value = big_payload();
    BlockedPublish publish(runtime, value);
    require_write_stuck(runtime, peer, expected_pub(value).size());
    peer.inject({mock::Action::text(msg("sub-1", 1))});
    {
        std::unique_lock<std::mutex> lock(mutex);
        CHECK(cv.wait_for(lock, milliseconds(5000), [&] { return entered; }));
    }
    // One handler running, one queued, the third overflows.
    peer.inject({mock::Action::text(msg("sub-1", 2)), mock::Action::text(msg("sub-1", 3))});
    publish.join();
    CHECK(publish.status.error().kind == ErrorKind::delivery_overflow);
    // The peer never drains: the deferred close is cut short by close_timeout.
    CHECK(io_until(runtime, transport_finished).has_value());
    CHECK_EQ(runtime.stats().writes_aborted, 1U);
    {
        std::lock_guard<std::mutex> lock(mutex);
        release = true;
    }
    cv.notify_all();
    CHECK(check::eventually([&] { return runtime.stats().delivered == 2; }));
    CHECK(runtime.close().ok());
    CHECK(no_runtime_threads());
}

TEST_CASE(write_ownership_repeats_across_connections) {
    // Many short-lived connections, each closed while its large write is
    // pending, alternating the error and success completions. Under ASan a
    // frame freed while Beast still masks from it is a reported error.
    for (int round = 0; round < 6; ++round) {
        const bool drain = round % 2 == 1;
        mock::Peer peer(mock::acknowledge_everything, mock::Peer::Handshake::owp, kPeerReceiveBuffer);
        peer.pause_reading_after(1);
        lacal::Config config = config_for(peer.url());
        config.close_timeout = drain ? milliseconds(20000) : milliseconds(200);
        lacal::Runtime runtime = connect_ok(config);
        CHECK(peer.wait_paused());
        const auto value = big_payload();
        BlockedPublish publish(runtime, value);
        CHECK(io_until(runtime, write_outstanding).has_value());
        std::thread closer([&] { CHECK(runtime.close().ok()); });
        CHECK(io_until(runtime, close_deferred_behind_write).has_value());
        if (drain) {
            peer.resume_reading();
        }
        closer.join();
        publish.join();
        CHECK(publish.status.error().kind == ErrorKind::connection_closed);
        CHECK_EQ(runtime.stats().writes_aborted, drain ? 0U : 1U);
    }
    CHECK(no_runtime_threads());
}

// ---- memory bounds ------------------------------------------------------------------

TEST_CASE(byte_bounds_are_validated) {
    mock::Peer peer;
    lacal::Config config = config_for(peer.url());
    config.max_pending_delivery_bytes = 0;
    CHECK(lacal::Runtime::connect(config).error().kind == ErrorKind::invalid_argument);
    config = config_for(peer.url());
    config.max_pending_event_bytes = 0;
    CHECK(lacal::Runtime::connect(config).error().kind == ErrorKind::invalid_argument);
    config = config_for(peer.url());
    config.max_message_bytes = 4096;
    config.max_pending_delivery_bytes = 4095;  // one admissible MSG would not fit
    CHECK(lacal::Runtime::connect(config).error().kind == ErrorKind::invalid_argument);
    CHECK_EQ(peer.connections(), 0);  // rejected before any I/O
    config.max_pending_delivery_bytes = 4096;
    CHECK(lacal::Runtime::connect(config).ok());
}

TEST_CASE(delivery_byte_bound_fails_closed) {
    mock::Peer peer;
    lacal::Config config = config_for(peer.url());
    config.max_message_bytes = 4096;
    config.max_pending_delivery_bytes = 4096;  // the count bound (4096 MSGs) is far away
    lacal::Runtime runtime = connect_ok(config);
    std::mutex mutex;
    std::condition_variable cv;
    bool entered = false;
    bool release = false;
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto&) {
        std::unique_lock<std::mutex> lock(mutex);
        entered = true;
        cv.notify_all();
        cv.wait(lock, [&] { return release; });
    });
    CHECK(subscription.ok());
    peer.inject({mock::Action::text(msg("sub-1", 1))});
    {
        std::unique_lock<std::mutex> lock(mutex);
        CHECK(cv.wait_for(lock, milliseconds(5000), [&] { return entered; }));
    }
    // The running delivery still counts. 1500-byte payloads: two fit, the
    // third crosses 4096 bytes and the connection fails closed.
    const std::string filler(1500, 'z');
    peer.inject({mock::Action::text("MSG sub-1 " + filler), mock::Action::text("MSG sub-1 " + filler),
                 mock::Action::text("MSG sub-1 " + filler)});
    auto event = runtime.poll_event(milliseconds(5000));
    CHECK(event && event->kind == EventKind::connection_closed);
    CHECK(event && event->error.kind == ErrorKind::delivery_overflow);
    CHECK(event && event->error.message.find("max_pending_delivery_bytes") != std::string::npos);
    CHECK(!runtime.connected());
    {
        std::lock_guard<std::mutex> lock(mutex);
        release = true;
    }
    cv.notify_all();
    // Everything admitted before the overflow is still dispatched, nothing silently.
    CHECK(check::eventually([&] { return runtime.stats().decode_errors == 2; }));
    CHECK_EQ(runtime.stats().delivered, 1U);
    CHECK(runtime.close().ok());
}

TEST_CASE(delivery_bytes_are_released_after_dispatch) {
    mock::Peer peer;
    lacal::Config config = config_for(peer.url());
    config.max_message_bytes = 4096;
    config.max_pending_delivery_bytes = 4096;
    lacal::Runtime runtime = connect_ok(config);
    std::atomic<int> calls{0};
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto&) { ++calls; });
    CHECK(subscription.ok());
    // 40 x ~60 bytes in sequence never exceeds the bound because each
    // dispatched delivery returns its bytes.
    for (int i = 0; i < 40; ++i) {
        peer.inject({mock::Action::text(msg("sub-1", i))});
        const int expected = i + 1;
        CHECK(check::eventually([&] { return calls.load() == expected; }));
    }
    // 2000-byte messages one after another: each is released when dispatched.
    for (std::uint64_t i = 0; i < 20; ++i) {
        peer.inject({mock::Action::text("MSG nobody " + std::string(2000, 'q'))});
        CHECK(check::eventually([&] { return runtime.stats().unknown_subscription == i + 1; }));
    }
    CHECK(runtime.connected());
    CHECK(runtime.close().ok());
}

TEST_CASE(event_byte_bound_omits_payloads_then_drops) {
    mock::Peer peer;
    lacal::Config config = config_for(peer.url());
    config.max_pending_event_bytes = 2048;
    lacal::Runtime runtime = connect_ok(config);
    const std::string body(1500, 'u');
    peer.inject({mock::Action::text("MSG nobody " + body), mock::Action::text("MSG nobody " + body),
                 mock::Action::text("MSG nobody " + body),
                 mock::Action::text("-ERR Internal-Error " + std::string(3000, 'd'))});
    CHECK(check::eventually([&] {
        const lacal::Stats s = runtime.stats();
        return s.unknown_subscription == 3 && s.events_dropped == 1;
    }));
    const lacal::Stats stats = runtime.stats();
    CHECK_EQ(stats.event_payloads_omitted, 2U);
    auto dropped = runtime.poll_event();
    CHECK(dropped && dropped->kind == EventKind::events_dropped && dropped->count == 1);
    auto first = runtime.poll_event();
    CHECK(first && first->kind == EventKind::unknown_subscription && first->payload == body);
    CHECK(first && !first->payload_omitted && first->payload_bytes == body.size());
    for (int i = 0; i < 2; ++i) {
        auto later = runtime.poll_event();
        CHECK(later && later->kind == EventKind::unknown_subscription && later->sid == "nobody");
        CHECK(later && later->payload_omitted && later->payload.empty() && later->payload_bytes == body.size());
    }
    CHECK(!runtime.poll_event().has_value());
    // Reading the events returned their budget.
    peer.inject({mock::Action::text("MSG nobody " + body)});
    auto again = runtime.poll_event(milliseconds(5000));
    CHECK(again && !again->payload_omitted && again->payload == body);
    CHECK(runtime.connected());
    CHECK(runtime.close().ok());
}

int main(int argc, char** argv) { return check::run_all("CPP_OMS_003_SHUTDOWN_COMPLETE", argc, argv); }
