// CPP-OMS-008: offline unit test of the standalone C++ UCI Log client's
// release-agnostic parts: CLI validation (before any connection), the
// bounded ordered queue, and the mapping of runtime outcomes to reports and
// exit statuses. No UCI schema, generated type, network or Sleet; compiled
// in PR smoke with strict C++17 flags against the runtime's public headers.
//
// Self-contained on purpose: the application must not depend on test-only
// helpers, and this test uses none either.
#include <atomic>
#include <chrono>
#include <cstdint>
#include <iostream>
#include <string>
#include <string_view>
#include <thread>
#include <vector>

#include "bounded_queue.hpp"
#include "cli.hpp"
#include "session.hpp"

namespace {

int g_checks = 0;
int g_failures = 0;

void check(bool condition, const char* what, int where) {
    ++g_checks;
    if (!condition) {
        ++g_failures;
        std::cerr << "unit_test.cpp:" << where << ": CHECK failed: " << what << '\n';
    }
}
#define CHECK(condition) check((condition), #condition, __LINE__)

namespace app = cpp_uci_client;
namespace lacal = ::ams_gra::lacal;
using Kind = app::ParseResult::Kind;

app::ParseResult parse(std::vector<std::string_view> args) { return app::parse_arguments(args); }

bool rejected(std::vector<std::string_view> args, std::string_view needle) {
    const app::ParseResult result = parse(std::move(args));
    return result.kind == Kind::error && result.error.find(needle) != std::string::npos;
}

void cli_valid() {
    const auto r = parse({"--url", "ws://127.0.0.1:4222", "--service-id", "cpp-uci-client"});
    CHECK(r.kind == Kind::run);
    CHECK(r.options.url == "ws://127.0.0.1:4222");
    CHECK(r.options.service_id == "cpp-uci-client");
    CHECK(r.options.duration_seconds == app::kDefaultDurationSeconds);
    CHECK(r.options.max_messages == app::kDefaultMaxMessages);
    const auto all = parse({"--max-messages", "100000", "--duration-seconds", "3600", "--service-id", "a.b_c-D9",
                            "--url", "WS://[::1]:9/owp?x=1"});
    CHECK(all.kind == Kind::run && all.options.max_messages == 100000U && all.options.duration_seconds == 3600U);
    CHECK(parse({"--url", "ws://host", "--service-id", "s", "--duration-seconds", "1", "--max-messages", "1"}).kind ==
          Kind::run);
    CHECK(parse({"--help"}).kind == Kind::help);
}

void cli_invalid() {
    CHECK(rejected({}, "required"));
    CHECK(rejected({"--service-id", "s"}, "--url is required"));
    CHECK(rejected({"--url", "ws://h:1"}, "--service-id is required"));
    CHECK(rejected({"--url"}, "requires a value"));
    CHECK(rejected({"--url", "--service-id", "s"}, "requires a value"));
    CHECK(rejected({"--url", "", "--service-id", "s"}, "requires a value"));
    CHECK(rejected({"--url=ws://h:1", "--service-id", "s"}, "unknown argument"));
    CHECK(rejected({"--topic", "uci.log"}, "unknown argument"));
    CHECK(rejected({"--schema", "002.6.0"}, "unknown argument"));  // the release is build-time only
    CHECK(rejected({"--url", "ws://h:1", "--url", "ws://h:2", "--service-id", "s"}, "only once"));
    CHECK(rejected({"--help", "--url", "ws://h:1"}, "only argument"));
    CHECK(rejected({"--url", "ws://h:1", "--service-id", "s", "--help"}, "only argument"));
    CHECK(rejected({"stray\x01", "x"}, "unknown argument 'stray?'"));
    for (std::string_view bad : {"0", "3601", "-1", "+5", "1.5", "x", "99999999999", " 5"}) {
        CHECK(rejected({"--url", "ws://h:1", "--service-id", "s", "--duration-seconds", bad}, "1..3600"));
    }
    for (std::string_view bad : {"0", "100001", "-3", "ten", "4294967296"}) {
        CHECK(rejected({"--url", "ws://h:1", "--service-id", "s", "--max-messages", bad}, "1..100000"));
    }
    for (std::string_view bad : {"bad id", "a/b", "a:b", "\xc3\xa4", "\x7f"}) {
        CHECK(rejected({"--url", "ws://h:1", "--service-id", bad}, "OWP identifier"));
    }
    const std::string too_long(129, 'a');
    const std::string longest(128, 'a');
    CHECK(rejected({"--url", "ws://h:1", "--service-id", too_long}, "OWP identifier"));
    CHECK(parse({"--url", "ws://h:1", "--service-id", longest}).kind == Kind::run);
}

void cli_urls() {
    const auto url = [](std::string_view u, std::string_view needle) {
        return rejected({"--url", u, "--service-id", "s"}, needle);
    };
    // Native WSS: an explicit, distinct refusal; never downgraded to ws://.
    CHECK(url("wss://127.0.0.1:4443", "native wss:// (TLS) is not supported"));
    CHECK(url("WSS://broker.example/owp", "native wss:// (TLS) is not supported"));
    CHECK(url("http://h:1", "unsupported URL scheme 'http://'"));
    CHECK(url("https://h:1", "unsupported URL scheme"));
    CHECK(url("h:1", "must start with ws://"));
    CHECK(url("ws://", "no host"));
    CHECK(url("ws://:9", "no host"));
    CHECK(url("ws:///path", "no host"));
    CHECK(url("ws://u:p@h:1", "user information"));
    CHECK(url("ws://h:1/#frag", "fragment"));
    CHECK(url("ws://h:0", "1..65535"));
    CHECK(url("ws://h:65536", "1..65535"));
    CHECK(url("ws://h:port", "1..65535"));
    CHECK(url("ws://h:", "1..65535"));
    CHECK(url("ws://[::1", "unterminated IPv6"));
    CHECK(url("ws://[::1]x", "invalid authority"));
    CHECK(url("ws://h :1", "whitespace"));
    CHECK(url("ws://h\t:1", "whitespace"));
    const std::string huge = "ws://" + std::string(2050, 'a');
    CHECK(url(huge, "longer than 2048"));
    CHECK(!app::check_url("ws://localhost").has_value());
    CHECK(!app::check_url("ws://127.0.0.1:65535/owp").has_value());
    const std::string help = app::help_text("cpp-uci-client", "2.6", "002.6.0", "Log", "uci.log");
    CHECK(help.find("BUILT FOR UCI 2.6") != std::string::npos);
    CHECK(help.find("OWP INIT schema 002.6.0") != std::string::npos);
    CHECK(help.find("native wss:// TLS is not supported") != std::string::npos);
    CHECK(help.find("cannot be changed at run time") != std::string::npos);
}

using Queue = app::BoundedQueue<int>;
using Clock = std::chrono::steady_clock;

void queue_bounds_and_order() {
    Queue queue(3);
    CHECK(queue.capacity() == 3U);
    int out = -1;
    CHECK(queue.pop_until(Clock::now() + std::chrono::milliseconds(5), out) == Queue::Pop::timeout);
    for (int i = 1; i <= 3; ++i) {
        CHECK(queue.push(int{i}) == Queue::Push::accepted);
    }
    CHECK(queue.size() == 3U);
    CHECK(queue.push(4) == Queue::Push::full);  // the explicit bound
    CHECK(queue.overflowed());
    // Overflow is sticky and reported before any further item: no silent gap.
    CHECK(queue.pop_until(Clock::now(), out) == Queue::Pop::overflowed);
    CHECK(queue.push(5) == Queue::Push::full);
    const auto counters = queue.counters();
    CHECK(counters.accepted == 3U && counters.rejected_full == 2U && counters.high_water == 3U);

    Queue fifo(4);
    for (int i = 10; i < 14; ++i) {
        CHECK(fifo.push(int{i}) == Queue::Push::accepted);
    }
    for (int i = 10; i < 14; ++i) {
        CHECK(fifo.pop_until(Clock::now(), out) == Queue::Pop::item && out == i);
    }
    CHECK(fifo.pop_until(Clock::now(), out) == Queue::Pop::timeout);
    CHECK(Queue(0).capacity() == 1U);
}

void queue_close_with_pending() {
    Queue queue(8);
    for (int i = 0; i < 5; ++i) {
        CHECK(queue.push(int{i}) == Queue::Push::accepted);
    }
    CHECK(queue.close() == 5U);  // pending items discarded and counted
    CHECK(queue.close() == 0U);  // idempotent
    CHECK(queue.push(9) == Queue::Push::closed);
    int out = -1;
    CHECK(queue.pop_until(Clock::now() + std::chrono::seconds(5), out) == Queue::Pop::closed);
    const auto counters = queue.counters();
    CHECK(counters.discarded_on_close == 5U && counters.rejected_closed == 1U && queue.size() == 0U);
    // close() wakes a waiting consumer promptly.
    Queue waiting(2);
    std::thread closer([&] {
        std::this_thread::sleep_for(std::chrono::milliseconds(30));
        (void)waiting.close();
    });
    const auto started = Clock::now();
    CHECK(waiting.pop_until(started + std::chrono::seconds(10), out) == Queue::Pop::closed);
    CHECK(Clock::now() - started < std::chrono::seconds(5));
    closer.join();
}

void queue_concurrent_order() {
    constexpr int kCount = 20000;
    Queue queue(64);
    std::atomic<int> full{0};
    std::thread producer([&] {
        for (int i = 0; i < kCount;) {
            if (queue.push(int{i}) == Queue::Push::accepted) {
                ++i;
            } else {
                ++full;
                break;  // overflow is terminal by design
            }
        }
    });
    int expected = 0;
    int out = -1;
    bool in_order = true;
    for (;;) {
        const Queue::Pop pop = queue.pop_until(Clock::now() + std::chrono::milliseconds(200), out);
        if (pop == Queue::Pop::item) {
            in_order = in_order && out == expected;
            ++expected;
            continue;
        }
        if (pop == Queue::Pop::overflowed || (pop == Queue::Pop::timeout && expected == kCount)) {
            break;
        }
    }
    producer.join();
    CHECK(in_order);  // FIFO across threads
    CHECK(expected == kCount || (full.load() == 1 && queue.overflowed()));
}

lacal::Error error(lacal::ErrorKind kind, std::string message, std::string code = {}, std::string details = {}) {
    lacal::Error e;
    e.kind = kind;
    e.message = std::move(message);
    e.owp_code = std::move(code);
    e.owp_details = std::move(details);
    return e;
}

void outcomes() {
    namespace status = app::exit_status;
    using K = lacal::ErrorKind;
    // Connect failures keep the runtime's own classification in the report.
    CHECK(app::connect_exit_status(error(K::connect_failed, "TCP connect failed")) == status::connect);
    CHECK(app::connect_exit_status(error(K::handshake_rejected, "no owp")) == status::connect);
    CHECK(app::connect_exit_status(error(K::broker_rejected, "INIT", "Unsupported-Schema")) == status::connect);
    CHECK(app::connect_exit_status(error(K::timeout, "connect")) == status::connect);
    CHECK(app::connect_exit_status(error(K::protocol_violation, "INFO")) == status::connect);
    CHECK(app::connect_exit_status(error(K::unsupported_scheme, "wss")) == status::usage);
    CHECK(app::subscribe_exit_status(error(K::broker_rejected, "-ERR", "Illegal-State")) == status::subscribe_rejected);
    CHECK(app::subscribe_exit_status(error(K::timeout, "SUB")) == status::session);
    CHECK(app::subscribe_exit_status(error(K::connection_closed, "lost")) == status::session);
    const std::string described =
        app::describe_error(error(K::broker_rejected, "-ERR x", "Illegal-State", "not authorized\x01"));
    CHECK(described.find("kind=broker_rejected owp=Illegal-State") == 0);
    CHECK(described.find("not authorized?") != std::string::npos);  // control characters neutralized

    // Decoder rejection: reported by codec path, never fatal, payload hidden.
    lacal::Event decode;
    decode.kind = lacal::EventKind::decode_error;
    decode.payload = R"({"Log":{"SECRET-CONTENT":1}})";
    decode.payload_bytes = decode.payload.size();
    decode.error = error(K::codec, "not hexBinary: odd number of hexadecimal digits");
    decode.error.codec_path = "LogMT.MessageData.Data";
    auto outcome = app::describe_event(decode);
    CHECK(!outcome.fatal);
    CHECK(outcome.line.find("DECODE-ERROR") == 0);
    CHECK(outcome.line.find("path=LogMT.MessageData.Data") != std::string::npos);
    CHECK(outcome.line.find("odd number") != std::string::npos);
    CHECK(outcome.line.find("SECRET") == std::string::npos);
    // A diagnostic that quotes document content is withheld.
    decode.error.message = "unknown member \"SECRET-CONTENT\"";
    outcome = app::describe_event(decode);
    CHECK(outcome.line.find("SECRET") == std::string::npos);
    CHECK(outcome.line.find("withheld") != std::string::npos);

    lacal::Event lost;
    lost.kind = lacal::EventKind::connection_closed;
    lost.error = error(K::connection_closed, "peer closed");
    CHECK(app::describe_event(lost).fatal);
    CHECK(app::describe_event(lost).line.find("CONNECTION-LOST kind=connection_closed") == 0);
    lacal::Event thrown;
    thrown.kind = lacal::EventKind::handler_exception;
    thrown.error = error(K::none, "handler or codec threw: boom");
    CHECK(app::describe_event(thrown).fatal);
    CHECK(app::describe_event(thrown).line.find("CALLBACK-FAILURE") == 0);
    lacal::Event unknown;
    unknown.kind = lacal::EventKind::unknown_subscription;
    unknown.sid = "sub-1";
    unknown.payload = "SECRET";
    CHECK(!app::describe_event(unknown).fatal);
    CHECK(app::describe_event(unknown).line.find("SECRET") == std::string::npos);
    lacal::Event server;
    server.kind = lacal::EventKind::server_error;
    CHECK(!app::describe_event(server).fatal);
    lacal::Event dropped;
    dropped.kind = lacal::EventKind::events_dropped;
    dropped.count = 3;
    CHECK(app::describe_event(dropped).line == "EVENTS-DROPPED count=3 (runtime event queue full)");
    CHECK(app::info_server_id(R"({"version":"1.0","server_id":"cpp-oms-008"})") == "cpp-oms-008");
    CHECK(app::info_server_id(R"({"version":"1.0"})") == "<unparsed>");
}

}  // namespace

int main() {
    cli_valid();
    cli_invalid();
    cli_urls();
    queue_bounds_and_order();
    queue_close_with_pending();
    queue_concurrent_order();
    outcomes();
    if (g_failures != 0) {
        std::cerr << g_failures << " of " << g_checks << " checks failed\n";
        return 1;
    }
    std::cout << "CPP_OMS_008_CLIENT_UNIT_COMPLETE checks=" << g_checks << '\n';
    return 0;
}
