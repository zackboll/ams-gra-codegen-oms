// CPP-OMS-008 TEST-ONLY: the application's receiver core (src/client.hpp,
// production-generated LogMT/endpoint/codec, the unchanged C++ LA-CAL
// runtime) against the UNMODIFIED pinned Sleet, for cases that need
// controlled internals: a small queue capacity, a throwing extractor, direct
// runtime access for the wrong global-element name, and resource counts.
//
//   CPP_UCI_CLIENT_SLEET_URL=ws://127.0.0.1:PORT receiver_test
//
// Independent traffic comes ONLY from tests/cpp-lacal/raw_owp_client.hpp with
// hand-written documents (documents.hpp); the generated encoder is unused.
#include <chrono>
#include <cstdlib>
#include <iostream>
#include <optional>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

#include "check.hpp"
#include "client.hpp"
#include "documents.hpp"
#include "raw_owp_client.hpp"

namespace app = cpp_uci_client;
namespace lacal = ::ams_gra::lacal;
namespace rel = cpp_uci_client::release;
using std::chrono::milliseconds;

namespace {

const std::string kSchema(rel::schema_version);
const std::string kService = "cpp-uci-client";
const std::string kWire = "cpp-uci-wire";
const milliseconds kWait(10000);
const milliseconds kQuiet(400);

std::string url() {
    const char* value = std::getenv("CPP_UCI_CLIENT_SLEET_URL");
    if (value == nullptr || *value == '\0') {
        std::cerr << "CPP_UCI_CLIENT_SLEET_URL is required\n";
        std::exit(2);
    }
    return value;
}

std::string host() {
    const std::string u = url();
    const std::string rest = u.substr(u.find("://") + 3);
    return rest.substr(0, rest.rfind(':'));
}

std::string port() {
    const std::string u = url();
    return u.substr(u.rfind(':') + 1);
}

raw::Client wire() { return raw::Client(host(), port(), kWire, kSchema, kWait); }

void publish(raw::Client& client, const std::string& body) {
    CHECK_EQ(client.command("PUB uci.log {\"Log\":" + body + "}"), std::string("+OK"));
}

app::ClientSettings settings(std::size_t capacity) {
    app::ClientSettings s;
    s.url = url();
    s.service_id = kService;
    s.queue_capacity = capacity;
    return s;
}

/// Connected and subscribed (SUB acknowledged) client.
void start(app::Client& client) {
    CHECK(!client.connect().has_value());
    CHECK(!client.subscribe().has_value());
    CHECK_EQ(client.subscription_id(), std::string("sub-1"));
}

void clean_shutdown(app::Client& client, std::size_t expected_discarded) {
    const app::ShutdownReport report = client.shutdown();
    CHECK(report.unsubscribe_ok);  // explicit UNSUB acknowledged by Sleet
    CHECK(report.close_ok);
    CHECK_EQ(report.pending_discarded, expected_discarded);
    CHECK_EQ(report.callbacks_after_unsubscribe, std::uint64_t{0});
    CHECK_EQ(report.runtime_threads, 0);
    CHECK(report.resources_released());
    CHECK_EQ(check::threads_named("ams-lacal"), 0);
}

}  // namespace

/// Collects up to `count` typed Logs within kWait; non-fatal events go to `notices`.
std::vector<app::Delivered> receive(app::Client& client, std::size_t count, std::vector<std::string>* notices) {
    std::vector<app::Delivered> items;
    const auto deadline = std::chrono::steady_clock::now() + kWait;
    while (items.size() < count && std::chrono::steady_clock::now() < deadline) {
        app::Next next = client.next(deadline);
        if (next.kind == app::Next::Kind::message) {
            items.push_back(std::move(next.item));
        } else if (next.kind == app::Next::Kind::notice && notices != nullptr) {
            notices->push_back(next.line);
        } else if (next.kind == app::Next::Kind::failure) {
            std::cerr << "unexpected failure: " << next.line << '\n';
            CHECK(false);
            break;
        }
    }
    return items;
}

/// The first non-idle outcome within kWait (next() itself waits at most ~25 ms).
app::Next next_outcome(app::Client& client) {
    const auto deadline = std::chrono::steady_clock::now() + kWait;
    app::Next next;
    while (std::chrono::steady_clock::now() < deadline) {
        next = client.next(deadline);
        if (next.kind != app::Next::Kind::idle) {
            break;
        }
    }
    return next;
}

TEST_CASE(typed_order_and_selected_fields) {
    raw::Client publisher = wire();  // opened first: part of the process's starting socket set
    app::Client client(settings(64));
    start(client);
    for (int n = 1; n <= 6; ++n) {
        publish(publisher, documents::variant(kSchema, n));
    }
    const auto items = receive(client, 6, nullptr);
    CHECK_EQ(items.size(), std::size_t{6});
    for (std::size_t i = 0; i < items.size(); ++i) {
        const int n = static_cast<int>(i) + 1;
        const app::LogSummary& s = items[i].summary;
        CHECK_EQ(items[i].sequence, static_cast<std::uint64_t>(n));  // subscription order preserved
        CHECK_EQ(std::string(app::severity_name(s.severity)), std::string(documents::variant_severity(n)));
        CHECK_EQ(std::string(app::classification_name(s.classification)), std::string("U"));
        CHECK_EQ(s.schema_version, kSchema);
        CHECK_EQ(s.header_timestamp, std::string("2026-01-02T03:04:05Z"));
        CHECK_EQ(s.log_timestamp, documents::variant_log_timestamp(n));
        CHECK_EQ(s.system_uuid, std::string("12345678-1234-1234-8234-123456789abc"));
        CHECK_EQ(s.service_uuid.has_value(), n % 2 == 1);
        CHECK_EQ(s.subsystem_uuid.has_value(), n % 2 == 1);
        CHECK(!s.header_service_uuid.has_value());
    }
    if (!items.empty()) {
        CHECK_EQ(items[0].summary.service_uuid.value_or(""), std::string("550e8400-e29b-41d4-a716-446655440108"));
        CHECK_EQ(items[0].summary.subsystem_uuid.value_or(""), std::string("550e8400-e29b-41d4-a716-446655440208"));
    }
    clean_shutdown(client, 0);
    std::cout << "CPP-OMS-008 " << kSchema << " typed order and fields: PASSED" << std::endl;
}

TEST_CASE(decoder_versus_broker_rejection) {
    raw::Client observer = wire();
    CHECK_EQ(observer.command("SUB wire Log uci.log"), std::string("+OK"));
    raw::Client publisher = wire();
    app::Client client(settings(64));
    start(client);
    // Broker-admitted, client-rejected: Sleet +OK and the MSG really crosses Sleet.
    const std::string bad = documents::decoder_rejected(kSchema);
    publish(publisher, bad);
    CHECK_EQ(observer.receive().value_or(""), "MSG wire {\"Log\":" + bad + "}");
    // A valid Log on the same subscription afterwards is received normally.
    publish(publisher, documents::variant(kSchema, 2));
    CHECK(observer.receive().has_value());
    std::vector<std::string> notices;
    const auto items = receive(client, 1, &notices);
    for (int i = 0; i < 20 && notices.empty(); ++i) {
        const app::Next next = client.next(std::chrono::steady_clock::now() + milliseconds(50));
        if (next.kind == app::Next::Kind::notice) {
            notices.push_back(next.line);
        }
    }
    CHECK_EQ(items.size(), std::size_t{1});
    CHECK(!items.empty() && items[0].sequence == 1U);  // the rejected MSG never reached the callback
    CHECK_EQ(notices.size(), std::size_t{1});
    const std::string expected = "DECODE-ERROR Log rejected by the client: codec path=LogMT.MessageData.Data";
    CHECK(!notices.empty() && notices[0].rfind(expected, 0) == 0);
    CHECK(!notices.empty() && notices[0].find("\"000\"") == std::string::npos);  // content not displayed
    // Broker rejections: -ERR to the publisher, no MSG, no event, no callback.
    const std::string denied = publisher.command("PUB uci.log {\"Log\":" + documents::broker_rejected(kSchema) + "}");
    CHECK(denied.rfind("-ERR Invalid-Message", 0) == 0 && denied.find("Severity") != std::string::npos);
    const std::string logmt = publisher.command("PUB uci.log {\"LogMT\":" + documents::golden(kSchema) + "}");
    CHECK(logmt.rfind("-ERR Invalid-Message", 0) == 0 && logmt.find("LogMT") != std::string::npos);
    CHECK(!observer.receive(kQuiet).has_value());
    CHECK(client.next(std::chrono::steady_clock::now() + kQuiet).kind == app::Next::Kind::idle);
    const app::ShutdownReport report = client.shutdown();
    CHECK_EQ(report.stats.decode_errors, std::uint64_t{1});
    CHECK_EQ(report.stats.delivered, std::uint64_t{1});
    CHECK(report.clean());
    std::cout << "CPP-OMS-008 " << kSchema << " decoder rejection then valid Log; broker rejections: PASSED"
              << std::endl;
}

TEST_CASE(wrong_global_element_name_is_refused_by_sleet) {
    // The generated endpoint uses "Log" (static_assert in release.hpp). The
    // payload type name "LogMT" as the OWP global message is refused by
    // Sleet itself (SUB -ERR), which is a broker rejection, not a decoder one.
    lacal::Config config;
    config.url = url();
    config.service_id = kService;
    config.schema_version = kSchema;
    auto runtime = lacal::Runtime::connect(config);
    CHECK(runtime.ok());
    if (!runtime.ok()) {
        return;
    }
    auto wrong = runtime->subscribe<::programs::oam::LogMT>(rel::oam_namespace, "LogMT", rel::application_topic,
                                                            std::nullopt, [](const ::programs::oam::LogMT&) {});
    CHECK(!wrong.ok() && wrong.error().kind == lacal::ErrorKind::broker_rejected);
    CHECK(!wrong.ok() && wrong.error().owp_code == "Illegal-Argument");
    CHECK(!wrong.ok() && wrong.error().owp_details.find("LogMT") != std::string::npos);
    if (!wrong.ok()) {
        std::cout << "CPP-OMS-008 " << kSchema << " SUB LogMT: " << app::describe_error(wrong.error()) << std::endl;
    }
    CHECK(runtime->connected());  // the -ERR fails only that operation
    CHECK(runtime->close().ok());
    CHECK_EQ(check::threads_named("ams-lacal"), 0);
    std::cout << "CPP-OMS-008 " << kSchema << " wrong global element name: PASSED" << std::endl;
}

TEST_CASE(bounded_queue_overflow_is_explicit) {
    // Capacity 3 and no consumer: the fourth typed delivery overflows; the
    // application fails the session instead of skipping a Log.
    raw::Client publisher = wire();
    app::Client client(settings(3));
    start(client);
    for (int n = 1; n <= 5; ++n) {
        publish(publisher, documents::variant(kSchema, n));
    }
    CHECK(check::eventually([&] { return client.receiver().callbacks() >= 5; }, kWait));
    CHECK(client.receiver().queue().overflowed());
    const app::Next next = next_outcome(client);
    CHECK(next.kind == app::Next::Kind::failure && next.exit_status == app::exit_status::session);
    CHECK(next.line.rfind("QUEUE-OVERFLOW application queue capacity=3", 0) == 0);
    const app::ShutdownReport report = client.shutdown();
    CHECK_EQ(report.queue.accepted, std::uint64_t{3});
    CHECK_EQ(report.queue.rejected_full, std::uint64_t{2});
    CHECK_EQ(report.queue.high_water, std::size_t{3});
    CHECK_EQ(report.pending_discarded, std::size_t{3});
    CHECK(report.clean());
    std::cout << "CPP-OMS-008 " << kSchema << " bounded queue overflow: PASSED" << std::endl;
}

TEST_CASE(shutdown_with_pending_messages_and_no_stale_callbacks) {
    raw::Client publisher = wire();
    app::Client client(settings(64));
    start(client);
    for (int n = 1; n <= 4; ++n) {
        publish(publisher, documents::variant(kSchema, n));
    }
    CHECK(check::eventually([&] { return client.receiver().queue().size() == 4; }, kWait));
    const app::ShutdownReport report = client.shutdown();  // nothing was consumed
    clean_shutdown(client, 4);
    CHECK_EQ(report.callbacks, std::uint64_t{4});
    // After UNSUB and close: further Logs reach Sleet but never a callback.
    for (int n = 5; n <= 7; ++n) {
        publish(publisher, documents::variant(kSchema, n));
    }
    std::this_thread::sleep_for(kQuiet);
    CHECK_EQ(client.receiver().callbacks(), std::uint64_t{4});
    CHECK_EQ(client.receiver().callbacks_after_unsubscribe(), std::uint64_t{0});
    CHECK_EQ(client.receiver().queue().counters().rejected_closed, std::uint64_t{0});
    std::cout << "CPP-OMS-008 " << kSchema << " shutdown with pending messages: PASSED" << std::endl;
}

TEST_CASE(callback_failure_is_reported_and_contained) {
    raw::Client publisher = wire();
    int calls = 0;
    app::Client client(settings(8), [&calls](const ::programs::oam::LogMT& log) -> app::LogSummary {
        if (++calls == 1) {
            throw std::runtime_error("application extractor failed");
        }
        return app::summarize(log);
    });
    start(client);
    publish(publisher, documents::variant(kSchema, 1));
    const app::Next next = next_outcome(client);
    CHECK(next.kind == app::Next::Kind::failure && next.exit_status == app::exit_status::session);
    CHECK(next.line.rfind("CALLBACK-FAILURE", 0) == 0);
    CHECK(next.line.find("application extractor failed") != std::string::npos);
    const app::ShutdownReport report = client.shutdown();
    CHECK_EQ(report.stats.handler_exceptions, std::uint64_t{1});
    CHECK(report.clean());
    std::cout << "CPP-OMS-008 " << kSchema << " callback failure: PASSED" << std::endl;
}

TEST_CASE(unauthorized_subscription_is_a_broker_rejection) {
    app::ClientSettings s = settings(8);
    s.service_id = "cpp-uci-unauthorized";  // known to Sleet, not authorized for uci.log
    app::Client client(s);
    CHECK(!client.connect().has_value());
    const auto failure = client.subscribe();
    CHECK(failure.has_value() && failure->exit_status == app::exit_status::subscribe_rejected);
    CHECK(failure.has_value() &&
          failure->line.rfind("SUBSCRIBE-REJECTED by the broker: kind=broker_rejected owp=Illegal-State", 0) == 0);
    const app::ShutdownReport report = client.shutdown();
    CHECK(!report.subscribed && report.close_ok && report.resources_released());
    std::cout << "CPP-OMS-008 " << kSchema << " unauthorized SUB: PASSED" << std::endl;
}

int main(int argc, char** argv) {
    return check::run_all(("CPP_OMS_008_RECEIVER_COMPLETE schema=" + kSchema).c_str(), argc, argv);
}
