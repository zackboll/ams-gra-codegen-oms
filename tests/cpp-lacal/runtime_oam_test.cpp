// CPP-OMS-003: OAM-namespace LA-CAL names through the GENERATED runtime-oam
// facade and codec (bare `MessageA`, never Clark notation), against the
// test-only controlled peer. Separate binary because every generated
// service_api.hpp / service_codec.hpp uses the same C++ namespaces.
#include <chrono>
#include <condition_variable>
#include <mutex>
#include <string>
#include <vector>

#include "ams_gra/lacal/runtime.hpp"
#include "check.hpp"
#include "mock_peer.hpp"
#include "oam.hpp"
#include "service_api.hpp"
#include "service_codec.hpp"

AMS_GRA_LACAL_PAYLOAD_CODEC(::programs::oam::SharedPayload, ::service_codec::encode_SharedPayload,
                            ::service_codec::decode_SharedPayload)

namespace lacal = ams_gra::lacal;
namespace loop = service_api::function_loop;
using std::chrono::milliseconds;

namespace {

::programs::oam::SharedPayload shared(std::int64_t count) {
    auto value = decltype(::programs::oam::SharedPayload::count)::create(count);
    if (!value) {
        std::exit(2);
    }
    return ::programs::oam::SharedPayload{*value};
}

}  // namespace

TEST_CASE(oam_namespace_uses_bare_local_names) {
    mock::Peer peer;
    lacal::Config config;
    config.url = peer.url();
    config.service_id = "runtime-oam";
    config.schema_version = "000.1.0";
    auto connected = lacal::Runtime::connect(config);
    CHECK(connected.ok());
    if (!connected.ok()) {
        return;
    }
    lacal::Runtime runtime = std::move(connected).value();

    std::mutex mutex;
    std::condition_variable cv;
    std::vector<std::int64_t> seen;
    auto subscription = loop::exchange_input_a::subscribe(runtime, [&](const loop::exchange_input_a::Payload& value) {
        std::lock_guard<std::mutex> lock(mutex);
        seen.push_back(value.count.value());
        cv.notify_all();
    });
    CHECK(subscription.ok());
    CHECK(loop::exchange_output_a::publish(runtime, shared(3)).ok());
    CHECK(loop::exchange_output_b::publish(runtime, shared(3)).ok());
    const auto frames = peer.frames();
    CHECK_EQ(frames.at(1), std::string("SUB sub-1 MessageA loop-topic"));
    CHECK_EQ(frames.at(2), std::string(R"(PUB loop-topic {"MessageA":{"Count":3}})"));
    CHECK_EQ(frames.at(3), std::string(R"(PUB loop-topic {"MessageB":{"Count":3}})"));

    // The Clark-notation spelling of an OAM name is a different global element.
    peer.inject({mock::Action::text(R"(MSG sub-1 {"{https://www.vdl.afrl.af.mil/programs/oam}MessageA":{"Count":1}})")});
    auto event = runtime.poll_event(milliseconds(5000));
    CHECK(event && event->kind == lacal::EventKind::decode_error);
    CHECK(event && event->message_name == "MessageA");
    peer.inject({mock::Action::text(R"(MSG sub-1 {"MessageA":{"Count":42}})")});
    {
        std::unique_lock<std::mutex> lock(mutex);
        CHECK(cv.wait_for(lock, milliseconds(5000), [&] { return !seen.empty(); }));
        CHECK(seen == std::vector<std::int64_t>{42});
    }
    CHECK(subscription->unsubscribe().ok());
    CHECK(runtime.close().ok());
}

int main(int argc, char** argv) { return check::run_all("CPP_OMS_003_OAM_COMPLETE", argc, argv); }
