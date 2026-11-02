// CPP-OMS-003 Stage A/B: the C++ LA-CAL runtime driven ONLY through the
// GENERATED service API (runtime-test.yaml, namespace urn:test) and the
// GENERATED service_codec.hpp, against the test-only controlled peer.
//
// Build: see scripts/check-cpp-oms-003-runtime.sh (generated headers are put
// on the include path; nothing generated is committed).
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <mutex>
#include <optional>
#include <stdexcept>
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
namespace misconfigured = service_api::function_misconfigured;
using lacal::ErrorKind;
using lacal::EventKind;
using std::chrono::milliseconds;
using Clock = std::chrono::steady_clock;

namespace {

const std::string kInit = R"(INIT {"versions":["1.0"],"schema":"000.1.0","verbose":true,"service_id":"svc-1"})";

lacal::Config config_for(const std::string& url) {
    lacal::Config config;
    config.url = url;
    config.service_id = "svc-1";
    config.schema_version = "000.1.0";
    config.connect_timeout = milliseconds(3000);
    config.operation_timeout = milliseconds(3000);
    config.close_timeout = milliseconds(1000);
    return config;
}

lacal::Runtime connect_ok(const lacal::Config& config) {
    auto runtime = lacal::Runtime::connect(config);
    if (!runtime.ok()) {
        std::cerr << "test setup: connect failed: " << lacal::to_string(runtime.error().kind) << ": "
                  << runtime.error().message << '\n';
        std::exit(2);
    }
    return std::move(runtime).value();
}

lacal::Runtime connect_ok(mock::Peer& peer) { return connect_ok(config_for(peer.url())); }

::urn::test::SharedPayload shared(std::int64_t count) {
    auto value = decltype(::urn::test::SharedPayload::count)::create(count);
    if (!value) {
        std::exit(2);
    }
    return ::urn::test::SharedPayload{*value};
}

std::string msg(const std::string& sid, std::int64_t count, const std::string& element = "{urn:test}MessageA") {
    return "MSG " + sid + " {\"" + element + "\":{\"{urn:test}Count\":" + std::to_string(count) + "}}";
}

/// Thread-safe record of typed handler invocations.
struct Inbox {
    std::mutex mutex;
    std::condition_variable cv;
    std::vector<std::int64_t> counts;

    void add(const ::urn::test::SharedPayload& value) {
        {
            std::lock_guard<std::mutex> lock(mutex);
            counts.push_back(value.count.value());
        }
        cv.notify_all();
    }
    bool wait(std::size_t n, milliseconds timeout = milliseconds(5000)) {
        std::unique_lock<std::mutex> lock(mutex);
        return cv.wait_for(lock, timeout, [&] { return counts.size() >= n; });
    }
    std::vector<std::int64_t> snapshot() {
        std::lock_guard<std::mutex> lock(mutex);
        return counts;
    }
};

std::optional<lacal::Event> next_event(lacal::Runtime& runtime, milliseconds timeout = milliseconds(5000)) {
    return runtime.poll_event(timeout);
}

bool no_runtime_threads() {
    return check::eventually([] { return check::threads_named("ams-lacal") == 0; }, milliseconds(2000));
}

}  // namespace

// ---- Stage A/B: happy path through the generated facade ------------------------

TEST_CASE(generated_facade_round_trips_through_mock_peer) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    CHECK(runtime.connected());
    CHECK(peer.wait_frames(1));
    CHECK_EQ(peer.frames().at(0), kInit);
    CHECK_EQ(peer.requested_subprotocol(), std::string("owp"));
    CHECK(runtime.server_info().find("\"server_id\":\"mock-peer\"") != std::string::npos);
    CHECK_EQ(check::threads_named("ams-lacal"), 2);

    Inbox inbox;
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const track::exchange_input_a::Payload& value) {
        inbox.add(value);
    });
    CHECK(subscription.ok());
    CHECK_EQ(subscription->id(), std::string("sub-1"));
    CHECK(subscription->active());
    CHECK_EQ(peer.frames().at(1), std::string("SUB sub-1 {urn:test}MessageA input-topic"));

    lacal::Status published = track::exchange_output_b::publish(runtime, shared(7));
    CHECK(published.ok());
    CHECK_EQ(peer.frames().at(2), std::string(R"(PUB output-topic {"{urn:test}MessageB":{"{urn:test}Count":7}})"));

    peer.inject({mock::Action::text(msg("sub-1", 42))});
    CHECK(inbox.wait(1));
    CHECK(inbox.snapshot() == std::vector<std::int64_t>{42});

    CHECK(subscription->unsubscribe().ok());
    CHECK(!subscription->active());
    CHECK_EQ(peer.frames().at(3), std::string("UNSUB sub-1"));
    CHECK(!runtime.poll_event(milliseconds(50)).has_value());

    CHECK(runtime.close().ok());
    CHECK(!runtime.connected());
    CHECK(peer.wait_disconnect());
    CHECK(peer.client_sent_close());
    CHECK(no_runtime_threads());
    CHECK(runtime.close().ok());  // idempotent
    const lacal::Stats stats = runtime.stats();
    CHECK_EQ(stats.delivered, 1U);
    CHECK_EQ(stats.decode_errors, 0U);
}

TEST_CASE(same_payload_different_message_differs_on_the_wire) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    CHECK(track::exchange_output_a::publish(runtime, shared(7)).ok());
    CHECK(track::exchange_output_b::publish(runtime, shared(7)).ok());
    ::urn::test::OtherPayload other{"tag"};
    CHECK(track::exchange_output_c::publish(runtime, other).ok());
    const auto frames = peer.frames();
    CHECK_EQ(frames.size(), 4U);
    CHECK_EQ(frames.at(1), std::string(R"(PUB output-topic {"{urn:test}MessageA":{"{urn:test}Count":7}})"));
    CHECK_EQ(frames.at(2), std::string(R"(PUB output-topic {"{urn:test}MessageB":{"{urn:test}Count":7}})"));
    CHECK_EQ(frames.at(3), std::string(R"(PUB output-topic {"{urn:test}MessageC":{"{urn:test}Label":"tag"}})"));
}

TEST_CASE(subscription_group_is_on_the_wire_or_absent) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    auto grouped = track::exchange_input_a_grouped::subscribe(runtime, [](const auto&) {});
    auto plain = track::exchange_input_a::subscribe(runtime, [](const auto&) {});
    CHECK(grouped.ok() && plain.ok());
    CHECK_EQ(peer.frames().at(1), std::string("SUB sub-1 {urn:test}MessageA input-topic worker-group-1"));
    CHECK_EQ(peer.frames().at(2), std::string("SUB sub-2 {urn:test}MessageA input-topic"));
}

TEST_CASE(invalid_routing_metadata_is_a_local_error_not_a_frame) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    lacal::Status bad_topic = misconfigured::exchange_output_bad_topic::publish(runtime, shared(1));
    CHECK(bad_topic.error().kind == ErrorKind::invalid_argument);
    CHECK(bad_topic.error().message.find("bad topic") != std::string::npos);
    auto bad_group = misconfigured::exchange_input_bad_group::subscribe(runtime, [](const auto&) {});
    CHECK(!bad_group.ok());
    CHECK(bad_group.error().kind == ErrorKind::invalid_argument);
    CHECK(bad_group.error().message.find("pool 7") != std::string::npos);
    std::this_thread::sleep_for(milliseconds(50));
    CHECK_EQ(peer.frames().size(), 1U);  // only INIT
    auto good = track::exchange_input_a::subscribe(runtime, [](const auto&) {});
    CHECK(good.ok() && good->id() == "sub-2");  // the rejected SUB consumed sub-1
    // A QName that cannot be represented on the wire is rejected locally too.
    lacal::Status bad_name = runtime.publish("urn:test", "Message A", "output-topic", shared(1));
    CHECK(bad_name.error().kind == ErrorKind::invalid_argument);
}

TEST_CASE(codec_rejection_sends_nothing) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    ::urn::test::OtherPayload invalid{std::string("\xC3\x28", 2)};  // ill-formed UTF-8
    lacal::Status status = track::exchange_output_c::publish(runtime, invalid);
    CHECK(status.error().kind == ErrorKind::codec);
    CHECK(!status.error().codec_path.empty());
    std::this_thread::sleep_for(milliseconds(50));
    CHECK_EQ(peer.frames().size(), 1U);
    CHECK(runtime.connected());
}

// ---- broker rejection versus local rejection ------------------------------------

TEST_CASE(broker_rejection_of_pub_is_returned_to_the_publisher) {
    mock::Peer peer([](const std::string& frame) -> std::vector<mock::Action> {
        if (frame.rfind("PUB ", 0) == 0) {
            return {mock::Action::text("-ERR Illegal-State topic not allowed")};
        }
        return mock::acknowledge_everything(frame);
    });
    lacal::Runtime runtime = connect_ok(peer);
    lacal::Status status = track::exchange_output_a::publish(runtime, shared(3));
    CHECK(status.error().kind == ErrorKind::broker_rejected);
    CHECK_EQ(status.error().owp_code, std::string("Illegal-State"));
    CHECK_EQ(status.error().owp_details, std::string("topic not allowed"));
    CHECK(runtime.connected());
    CHECK(!runtime.poll_event(milliseconds(50)).has_value());  // paired, so not also an event
    peer.set_responder(mock::acknowledge_everything);
    CHECK(track::exchange_output_a::publish(runtime, shared(4)).ok());
}

TEST_CASE(broker_rejection_of_sub_registers_nothing) {
    mock::Peer peer([](const std::string& frame) -> std::vector<mock::Action> {
        if (frame.rfind("SUB ", 0) == 0) {
            return {mock::Action::text("-ERR Illegal-State not authorized to subscribe")};
        }
        return mock::acknowledge_everything(frame);
    });
    lacal::Runtime runtime = connect_ok(peer);
    std::atomic<int> calls{0};
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto&) { ++calls; });
    CHECK(!subscription.ok());
    CHECK(subscription.error().kind == ErrorKind::broker_rejected);
    peer.inject({mock::Action::text(msg("sub-1", 1))});
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::unknown_subscription && event->sid == "sub-1");
    CHECK_EQ(calls.load(), 0);
}

TEST_CASE(unpaired_server_error_is_an_event_and_connection_survives) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    peer.inject({mock::Action::text("-ERR Internal-Error broker hiccup")});
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::server_error);
    CHECK(event && event->error.owp_code == "Internal-Error" && event->error.owp_details == "broker hiccup");
    CHECK(runtime.connected());
    CHECK(track::exchange_output_a::publish(runtime, shared(1)).ok());
}

// ---- connect -------------------------------------------------------------------------

TEST_CASE(init_rejection_fails_connect) {
    mock::Peer peer([](const std::string&) -> std::vector<mock::Action> {
        return {mock::Action::text("-ERR Unsupported-Schema schema 000.1.0 not loaded")};
    });
    auto runtime = lacal::Runtime::connect(config_for(peer.url()));
    CHECK(!runtime.ok());
    CHECK(runtime.error().kind == ErrorKind::broker_rejected);
    CHECK_EQ(runtime.error().owp_code, std::string("Unsupported-Schema"));
    CHECK_EQ(runtime.error().owp_details, std::string("schema 000.1.0 not loaded"));
    CHECK(no_runtime_threads());
}

TEST_CASE(connect_input_errors_are_local) {
    mock::Peer peer;
    const auto expect = [&](lacal::Config config, ErrorKind kind, const char* fragment) {
        auto runtime = lacal::Runtime::connect(std::move(config));
        CHECK(!runtime.ok());
        CHECK(runtime.error().kind == kind);
        CHECK(runtime.error().message.find(fragment) != std::string::npos);
    };
    lacal::Config wss = config_for(peer.url());
    wss.url = "wss://127.0.0.1:" + std::to_string(peer.port());
    expect(wss, ErrorKind::unsupported_scheme, "TLS");
    lacal::Config http = config_for(peer.url());
    http.url = "http://127.0.0.1:" + std::to_string(peer.port());
    expect(http, ErrorKind::unsupported_scheme, "http");
    lacal::Config service = config_for(peer.url());
    service.service_id = "bad id";
    expect(service, ErrorKind::invalid_argument, "service_id");
    lacal::Config schema = config_for(peer.url());
    schema.schema_version.clear();
    expect(schema, ErrorKind::invalid_argument, "schema_version");
    lacal::Config version = config_for(peer.url());
    version.owp_versions = {"2.0"};
    expect(version, ErrorKind::invalid_argument, "2.0");
    lacal::Config port = config_for(peer.url());
    port.url = "ws://127.0.0.1:99999";
    expect(port, ErrorKind::invalid_argument, "port");
    lacal::Config user = config_for(peer.url());
    user.url = "ws://user@127.0.0.1:1";
    expect(user, ErrorKind::invalid_argument, "user");
    lacal::Config zero = config_for(peer.url());
    zero.operation_timeout = milliseconds(0);
    expect(zero, ErrorKind::invalid_argument, "timeouts");
    CHECK_EQ(peer.connections(), 0);  // nothing was even dialled
    CHECK(no_runtime_threads());
}

TEST_CASE(subprotocol_is_enforced_both_ways) {
    for (auto mode : {mock::Peer::Handshake::reject_400, mock::Peer::Handshake::omit_subprotocol,
                      mock::Peer::Handshake::wrong_subprotocol}) {
        mock::Peer peer(mock::acknowledge_everything, mode);
        auto runtime = lacal::Runtime::connect(config_for(peer.url()));
        CHECK(!runtime.ok());
        CHECK(runtime.error().kind == ErrorKind::handshake_rejected);
        CHECK_EQ(peer.requested_subprotocol(), std::string("owp"));
        if (mode == mock::Peer::Handshake::reject_400) {
            CHECK(runtime.error().message.find("400") != std::string::npos);
        } else {
            CHECK(runtime.error().message.find("'owp'") != std::string::npos);
        }
        CHECK(peer.frames().empty());  // INIT never sent over a wrong subprotocol
    }
    CHECK(no_runtime_threads());
}

TEST_CASE(connect_waits_are_bounded) {
    {
        mock::Peer peer(mock::acknowledge_everything, mock::Peer::Handshake::never_upgrade);
        lacal::Config config = config_for(peer.url());
        config.connect_timeout = milliseconds(300);
        const auto start = Clock::now();
        auto runtime = lacal::Runtime::connect(config);
        const auto elapsed = Clock::now() - start;
        CHECK(!runtime.ok() && runtime.error().kind == ErrorKind::timeout);
        CHECK(elapsed < milliseconds(2500));
    }
    {
        mock::Peer silent([](const std::string&) { return std::vector<mock::Action>{}; });
        lacal::Config config = config_for(silent.url());
        config.connect_timeout = milliseconds(300);
        auto runtime = lacal::Runtime::connect(config);
        CHECK(!runtime.ok() && runtime.error().kind == ErrorKind::timeout);
        CHECK_EQ(silent.frames().size(), 1U);
    }
    {
        unsigned short port = 0;
        {
            mock::Peer gone;
            port = gone.port();
        }
        lacal::Config config = config_for("ws://127.0.0.1:" + std::to_string(port));
        auto runtime = lacal::Runtime::connect(config);
        CHECK(!runtime.ok() && runtime.error().kind == ErrorKind::connect_failed);
    }
    {
        mock::Peer wrong_version([](const std::string&) {
            return std::vector<mock::Action>{mock::Action::text("+OK"),
                                              mock::Action::text(R"(INFO {"version":"9.9","server_id":"x"})")};
        });
        auto runtime = lacal::Runtime::connect(config_for(wrong_version.url()));
        CHECK(!runtime.ok() && runtime.error().kind == ErrorKind::protocol_violation);
        CHECK(!runtime.ok() && runtime.error().message.find("9.9") != std::string::npos);
    }
    {
        mock::Peer no_ok([](const std::string&) { return std::vector<mock::Action>{mock::Action::text(mock::kInfo)}; });
        auto runtime = lacal::Runtime::connect(config_for(no_ok.url()));
        CHECK(!runtime.ok() && runtime.error().kind == ErrorKind::protocol_violation);
    }
    CHECK(no_runtime_threads());
}

// ---- dispatch ----------------------------------------------------------------------

TEST_CASE(unknown_sid_invokes_no_handler) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    Inbox inbox;
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto& value) { inbox.add(value); });
    CHECK(subscription.ok());
    peer.inject({mock::Action::text(msg("unknown-id", 5))});
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::unknown_subscription);
    CHECK(event && event->sid == "unknown-id");
    CHECK(event && event->payload.find("\"{urn:test}Count\":5") != std::string::npos);
    CHECK(inbox.snapshot().empty());
    CHECK_EQ(runtime.stats().unknown_subscription, 1U);
}

TEST_CASE(malformed_payloads_are_rejected_and_subscriptions_survive) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    Inbox first;
    Inbox second;
    auto a = track::exchange_input_a::subscribe(runtime, [&](const auto& value) { first.add(value); });
    auto b = track::exchange_input_a_grouped::subscribe(runtime, [&](const auto& value) { second.add(value); });
    CHECK(a.ok() && b.ok());
    struct Case {
        std::string payload;
        bool codec_path;  // rejected by the generated codec (else by the envelope)
    };
    const std::vector<Case> cases = {
        {"{}", false},
        {R"({"{urn:test}MessageA":{"{urn:test}Count":1},"{urn:test}MessageB":{"{urn:test}Count":1}})", false},
        {R"({"{urn:test}MessageB":{"{urn:test}Count":1}})", false},
        {R"({"{urn:other}MessageA":{"{urn:test}Count":1}})", false},
        {R"({"MessageA":{"{urn:test}Count":1}})", false},
        {R"([{"{urn:test}MessageA":{"{urn:test}Count":1}}])", false},
        {"nope", false},
        {R"({"{urn:test}MessageA":{"{urn:test}Count":1}} trailing)", false},
        {R"({"{urn:test}MessageA":{"{urn:test}Count":"7"}})", true},
        {R"({"{urn:test}MessageA":{"{urn:test}Count":1.5}})", true},
        {R"({"{urn:test}MessageA":{"{urn:test}Count":2147483648}})", true},
        {R"({"{urn:test}MessageA":{}})", true},
        {R"({"{urn:test}MessageA":{"{urn:test}Count":1,"{urn:test}Extra":2}})", true},
        {R"({"{urn:test}MessageA":{"Count":1}})", true},
        {R"({"{urn:test}MessageA":[1]})", true},
        {R"({"{urn:test}MessageA":{"{urn:test}Count":1,"{urn:test}Count":2}})", true},
    };
    for (const Case& item : cases) {
        peer.inject({mock::Action::text("MSG sub-1 " + item.payload)});
        auto event = next_event(runtime);
        CHECK(event && event->kind == EventKind::decode_error);
        if (event) {
            CHECK_EQ(event->sid, std::string("sub-1"));
            CHECK_EQ(event->message_name, std::string("{urn:test}MessageA"));
            CHECK_EQ(event->payload, item.payload);
            CHECK(event->error.kind == ErrorKind::codec);
            CHECK_EQ(event->error.codec_path.empty(), !item.codec_path);
            if (!item.codec_path) {
                CHECK(event->error.message.rfind("envelope:", 0) == 0);
            }
        }
        // The same subscription still delivers, and the other is unaffected.
        peer.inject({mock::Action::text(msg("sub-1", 1)), mock::Action::text(msg("sub-2", 2))});
    }
    CHECK(first.wait(cases.size()));
    CHECK(second.wait(cases.size()));
    CHECK(first.snapshot() == std::vector<std::int64_t>(cases.size(), 1));
    CHECK(second.snapshot() == std::vector<std::int64_t>(cases.size(), 2));
    CHECK_EQ(runtime.stats().decode_errors, cases.size());
    CHECK(!runtime.poll_event(milliseconds(50)).has_value());
}

TEST_CASE(handler_exception_is_contained) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    Inbox inbox;
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto& value) {
        if (value.count.value() == 13) {
            throw std::runtime_error("unlucky");
        }
        inbox.add(value);
    });
    CHECK(subscription.ok());
    peer.inject({mock::Action::text(msg("sub-1", 13)), mock::Action::text(msg("sub-1", 14))});
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::handler_exception);
    CHECK(event && event->error.message.find("unlucky") != std::string::npos);
    CHECK(inbox.wait(1));
    CHECK(inbox.snapshot() == std::vector<std::int64_t>{14});
    CHECK_EQ(runtime.stats().handler_exceptions, 1U);
}

// ---- unsubscribe and ownership -----------------------------------------------------------

TEST_CASE(unsubscribe_removes_dispatch_and_is_one_shot) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    Inbox inbox;
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto& value) { inbox.add(value); });
    CHECK(subscription.ok());
    CHECK(subscription->unsubscribe().ok());
    CHECK(subscription->unsubscribe().error().kind == ErrorKind::subscription_inactive);
    peer.inject({mock::Action::text(msg("sub-1", 9))});
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::unknown_subscription);
    CHECK(inbox.snapshot().empty());
    {
        auto scoped = track::exchange_input_a::subscribe(runtime, [](const auto&) {});
        CHECK(scoped.ok() && scoped->id() == "sub-2");  // IDs are never reused
    }  // RAII: destroying the active handle unsubscribes
    CHECK(peer.wait_frames(5));
    CHECK_EQ(peer.frames().at(4), std::string("UNSUB sub-2"));
    // Moving a handle transfers ownership; the moved-from handle is inert.
    auto moved = track::exchange_input_a::subscribe(runtime, [](const auto&) {});
    CHECK(moved.ok());
    lacal::Subscription owner = std::move(moved.value());
    CHECK(!moved->active());
    CHECK(owner.active() && owner.id() == "sub-3");
    CHECK(owner.unsubscribe().ok());
}

TEST_CASE(unsubscribe_waits_for_the_running_handler) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    std::mutex mutex;
    std::condition_variable cv;
    bool entered = false;
    bool release = false;
    std::atomic<int> calls{0};
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto&) {
        ++calls;
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
    std::atomic<bool> returned{false};
    std::thread unsubscriber([&] {
        CHECK(subscription->unsubscribe().ok());
        returned = true;
    });
    std::this_thread::sleep_for(milliseconds(200));
    CHECK(!returned.load());  // still inside the handler
    {
        std::lock_guard<std::mutex> lock(mutex);
        release = true;
    }
    cv.notify_all();
    unsubscriber.join();
    CHECK(returned.load());
    peer.inject({mock::Action::text(msg("sub-1", 2))});
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::unknown_subscription);
    CHECK_EQ(calls.load(), 1);
}

TEST_CASE(handlers_may_reenter_the_runtime_but_not_close_it) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    std::optional<lacal::Subscription> own;
    std::atomic<int> stage{0};
    lacal::Status publish_status;
    lacal::Status close_status;
    lacal::Status unsubscribe_status;
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto& value) {
        if (value.count.value() == 1) {
            publish_status = track::exchange_output_b::publish(runtime, shared(100));
            close_status = runtime.close();
            unsubscribe_status = own->unsubscribe();  // its own subscription, from its own handler
            stage = 1;
        }
    });
    CHECK(subscription.ok());
    own.emplace(std::move(subscription.value()));
    peer.inject({mock::Action::text(msg("sub-1", 1))});
    CHECK(check::eventually([&] { return stage.load() == 1; }));
    CHECK(publish_status.ok());
    CHECK(close_status.error().kind == ErrorKind::called_from_runtime_thread);
    CHECK(unsubscribe_status.ok());
    CHECK(runtime.connected());
    const auto frames = peer.frames();
    CHECK_EQ(frames.at(2), std::string(R"(PUB output-topic {"{urn:test}MessageB":{"{urn:test}Count":100}})"));
    CHECK_EQ(frames.at(3), std::string("UNSUB sub-1"));
    peer.inject({mock::Action::text(msg("sub-1", 1))});
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::unknown_subscription);
}

TEST_CASE(subscription_may_outlive_its_runtime) {
    mock::Peer peer;
    std::optional<lacal::Subscription> survivor;
    {
        lacal::Runtime runtime = connect_ok(peer);
        auto subscription = track::exchange_input_a::subscribe(runtime, [](const auto&) {});
        CHECK(subscription.ok());
        survivor.emplace(std::move(subscription.value()));
    }  // Runtime destroyed (closed) first
    CHECK(no_runtime_threads());
    CHECK(survivor->unsubscribe().error().kind == ErrorKind::connection_closed);
    survivor.reset();  // destructor is a no-op now
}

// ---- failure: timeouts, loss, protocol violations, bounds ------------------------------

TEST_CASE(missing_acknowledgment_times_out_and_fails_the_connection) {
    mock::Peer peer([](const std::string& frame) -> std::vector<mock::Action> {
        if (frame.rfind("PUB ", 0) == 0) {
            return {};  // never acknowledged
        }
        return mock::acknowledge_everything(frame);
    });
    lacal::Config config = config_for(peer.url());
    config.operation_timeout = milliseconds(300);
    lacal::Runtime runtime = connect_ok(config);
    const auto start = Clock::now();
    lacal::Status status = track::exchange_output_a::publish(runtime, shared(1));
    CHECK(status.error().kind == ErrorKind::timeout);
    CHECK(Clock::now() - start < milliseconds(2000));
    CHECK(!runtime.connected());
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::connection_closed && event->error.kind == ErrorKind::timeout);
    CHECK(track::exchange_output_a::publish(runtime, shared(2)).error().kind == ErrorKind::connection_closed);
    CHECK(peer.wait_disconnect());
    CHECK(runtime.close().ok());
    CHECK(no_runtime_threads());
}

TEST_CASE(connection_loss_is_explicit) {
    mock::Peer peer([](const std::string& frame) -> std::vector<mock::Action> {
        if (frame.rfind("PUB ", 0) == 0 && frame.find("\"{urn:test}Count\":666") != std::string::npos) {
            return {mock::Action::drop()};
        }
        return mock::acknowledge_everything(frame);
    });
    lacal::Runtime runtime = connect_ok(peer);
    Inbox inbox;
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto& value) { inbox.add(value); });
    CHECK(subscription.ok());
    // A MSG that arrived before the loss is still delivered.
    peer.inject({mock::Action::text(msg("sub-1", 5))});
    CHECK(inbox.wait(1));
    // A command outstanding when the connection drops fails explicitly, quickly.
    const auto start = Clock::now();
    lacal::Status status = track::exchange_output_a::publish(runtime, shared(666));
    CHECK(status.error().kind == ErrorKind::connection_closed);
    CHECK(Clock::now() - start < milliseconds(2000));
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::connection_closed);
    CHECK(!runtime.connected());
    CHECK(track::exchange_output_a::publish(runtime, shared(1)).error().kind == ErrorKind::connection_closed);
    CHECK(subscription->unsubscribe().error().kind == ErrorKind::connection_closed);
    CHECK(runtime.close().ok());
    CHECK(no_runtime_threads());
}

TEST_CASE(peer_close_and_drop_while_idle_are_events) {
    for (const bool polite : {true, false}) {
        mock::Peer peer;
        lacal::Runtime runtime = connect_ok(peer);
        peer.inject({polite ? mock::Action::close() : mock::Action::drop()});
        auto event = next_event(runtime);
        CHECK(event && event->kind == EventKind::connection_closed);
        CHECK(event && event->error.kind == ErrorKind::connection_closed);
        if (polite && event) {
            CHECK(event->error.message.find("peer closed") != std::string::npos);
        }
        CHECK(!runtime.connected());
        CHECK(runtime.close().ok());
    }
    CHECK(no_runtime_threads());
}

TEST_CASE(protocol_violations_fail_closed) {
    const std::vector<mock::Action> violations = {
        mock::Action::binary("\x01\x02"),
        mock::Action::text("HELLO world"),
        mock::Action::text("+OK"),  // no outstanding command
        mock::Action::text("-ERR Made-Up-Code x"),
        mock::Action::text("MSG"),
    };
    for (const mock::Action& violation : violations) {
        mock::Peer peer;
        lacal::Runtime runtime = connect_ok(peer);
        peer.inject({violation});
        auto event = next_event(runtime);
        CHECK(event && event->kind == EventKind::connection_closed);
        CHECK(event && event->error.kind == ErrorKind::protocol_violation);
        CHECK(!runtime.connected());
        CHECK(peer.wait_disconnect());
    }
    CHECK(no_runtime_threads());
}

TEST_CASE(oversized_message_fails_closed) {
    mock::Peer peer;
    lacal::Config config = config_for(peer.url());
    config.max_message_bytes = 512;  // INFO (~230 bytes) fits; the MSG below does not
    lacal::Runtime runtime = connect_ok(config);
    peer.inject({mock::Action::text("MSG sub-1 {\"{urn:test}MessageA\":{\"pad\":\"" + std::string(1000, 'x') + "\"}}")});
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::connection_closed);
    CHECK(event && event->error.kind == ErrorKind::protocol_violation);
    CHECK(event && event->error.message.find("max_message_bytes") != std::string::npos);
}

TEST_CASE(delivery_backlog_overflow_is_explicit) {
    mock::Peer peer;
    lacal::Config config = config_for(peer.url());
    config.max_pending_deliveries = 2;
    lacal::Runtime runtime = connect_ok(config);
    std::mutex mutex;
    std::condition_variable cv;
    bool entered = false;
    bool release = false;
    std::atomic<int> calls{0};
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto&) {
        ++calls;
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
    // One handler is running; two fit in the backlog; the fourth overflows.
    peer.inject({mock::Action::text(msg("sub-1", 2)), mock::Action::text(msg("sub-1", 3)),
                 mock::Action::text(msg("sub-1", 4)), mock::Action::text(msg("sub-1", 5))});
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::connection_closed);
    CHECK(event && event->error.kind == ErrorKind::delivery_overflow);
    {
        std::lock_guard<std::mutex> lock(mutex);
        release = true;
    }
    cv.notify_all();
    CHECK(check::eventually([&] { return runtime.stats().delivered == 3; }));
    CHECK_EQ(calls.load(), 3);  // nothing queued before the overflow was lost
}

TEST_CASE(event_queue_bound_reports_drops) {
    mock::Peer peer;
    lacal::Config config = config_for(peer.url());
    config.max_pending_events = 2;
    lacal::Runtime runtime = connect_ok(config);
    std::vector<mock::Action> burst;
    for (int i = 0; i < 5; ++i) {
        burst.push_back(mock::Action::text(msg("nobody", i)));
    }
    peer.inject(burst);
    CHECK(check::eventually([&] { return runtime.stats().unknown_subscription == 5; }));
    auto dropped = runtime.poll_event();
    CHECK(dropped && dropped->kind == EventKind::events_dropped && dropped->count == 3);
    CHECK(runtime.poll_event() && runtime.poll_event());
    CHECK(!runtime.poll_event().has_value());
    CHECK_EQ(runtime.stats().events_dropped, 3U);
}

TEST_CASE(later_info_is_an_event) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
    peer.inject({mock::Action::text(mock::kInfo)});
    auto event = next_event(runtime);
    CHECK(event && event->kind == EventKind::info && event->payload.find("mock-peer") != std::string::npos);
}

// ---- concurrency: pairing under load ---------------------------------------------------------

TEST_CASE(concurrent_publishers_get_their_own_acknowledgments) {
    // The peer rejects every Count divisible by 3 (details = the count) and
    // interleaves a MSG before every 5th acknowledgment.
    std::atomic<int> seen{0};
    mock::Peer peer([&](const std::string& frame) -> std::vector<mock::Action> {
        if (frame.rfind("PUB ", 0) != 0) {
            return mock::acknowledge_everything(frame);
        }
        const std::size_t at = frame.find("Count\":");
        const long count = std::stol(frame.substr(at + 7));
        std::vector<mock::Action> actions;
        if (++seen % 5 == 0) {
            actions.push_back(mock::Action::text(msg("sub-1", count)));
        }
        actions.push_back(count % 3 == 0 ? mock::Action::text("-ERR Illegal-State " + std::to_string(count))
                                         : mock::Action::text("+OK"));
        return actions;
    });
    lacal::Runtime runtime = connect_ok(peer);
    std::atomic<int> delivered{0};
    auto subscription = track::exchange_input_a::subscribe(runtime, [&](const auto&) { ++delivered; });
    CHECK(subscription.ok());
    constexpr int kThreads = 4;
    constexpr int kEach = 60;
    std::atomic<int> wrong{0};
    std::vector<std::thread> threads;
    for (int t = 0; t < kThreads; ++t) {
        threads.emplace_back([&, t] {
            for (int i = 0; i < kEach; ++i) {
                const int count = t * 1000 + i;
                lacal::Status status = track::exchange_output_a::publish(runtime, shared(count));
                const bool should_fail = count % 3 == 0;
                if (should_fail != !status.ok() ||
                    (should_fail && status.error().owp_details != std::to_string(count))) {
                    ++wrong;
                }
            }
        });
    }
    for (auto& thread : threads) {
        thread.join();
    }
    CHECK_EQ(wrong.load(), 0);
    CHECK(check::eventually([&] { return delivered.load() == kThreads * kEach / 5; }));
    CHECK(runtime.connected());
    CHECK(!runtime.poll_event(milliseconds(50)).has_value());
}

// ---- shutdown -----------------------------------------------------------------------------------

TEST_CASE(close_and_destroy_leave_no_threads) {
    for (int run = 0; run < 3; ++run) {
        mock::Peer peer;
        {
            lacal::Runtime runtime = connect_ok(peer);
            auto subscription = track::exchange_input_a::subscribe(runtime, [](const auto&) {});
            CHECK(subscription.ok());
            CHECK(track::exchange_output_a::publish(runtime, shared(run)).ok());
            if (run == 0) {
                CHECK(runtime.close().ok());
            } else if (run == 1) {
                lacal::Runtime moved = std::move(runtime);
                CHECK(!runtime.connected());  // moved-from is inert
                CHECK(runtime.close().ok());
                CHECK(moved.connected());
            }
            // run 2: destructor only (subscription handle destroyed first)
        }
        CHECK(peer.wait_disconnect());
        CHECK(no_runtime_threads());
    }
}

TEST_CASE(close_discards_and_counts_the_undispatched_backlog) {
    mock::Peer peer;
    lacal::Runtime runtime = connect_ok(peer);
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
    // Frames are processed in order, so once the trailing unpaired -ERR is
    // visible as an event both MSGs are already in the backlog.
    peer.inject({mock::Action::text(msg("sub-1", 2)), mock::Action::text(msg("sub-1", 3)),
                 mock::Action::text("-ERR Internal-Error marker")});
    auto marker = next_event(runtime);
    CHECK(marker && marker->kind == EventKind::server_error);
    std::thread closer([&] { CHECK(runtime.close().ok()); });
    std::this_thread::sleep_for(milliseconds(100));
    {
        std::lock_guard<std::mutex> lock(mutex);
        release = true;
    }
    cv.notify_all();
    closer.join();
    const lacal::Stats stats = runtime.stats();
    CHECK_EQ(stats.delivered, 1U);
    CHECK_EQ(stats.discarded_on_close, 2U);
    CHECK(no_runtime_threads());
}

int main(int argc, char** argv) { return check::run_all("CPP_OMS_003_MOCK_COMPLETE", argc, argv); }
