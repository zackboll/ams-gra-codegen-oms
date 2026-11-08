// Authentic CPP-OMS-007 traffic through the existing LA-CAL runtime and Sleet.
#include "ams_gra/lacal/runtime.hpp"
#include "check.hpp"
#include "file_metadata_documents.hpp"
#include "service_api.hpp"
#include <condition_variable>
#include <deque>
#include <mutex>

AMS_GRA_LACAL_PAYLOAD_CODEC(::programs::oam::FileMetadataMT, ::service_codec::encode_FileMetadataMT,
                            ::service_codec::decode_FileMetadataMT)
namespace fm = file_metadata;
namespace lacal = ams_gra::lacal;
namespace svc = service_api::function_loop;
using std::chrono::milliseconds;
void roundtrip(bool include_hash) {
    const char* url = std::getenv("CPP_OMS_007_SLEET_URL");
    CHECK(url != nullptr);
    if (url == nullptr) return;
    lacal::Config config;
    config.url = url;
    config.service_id = "cpp-oms-007";
    config.schema_version = fm::version;
    config.connect_timeout = milliseconds(10000);
    config.operation_timeout = milliseconds(10000);
    auto connected = lacal::Runtime::connect(config);
    CHECK(connected.ok());
    if (!connected.ok()) { std::cerr << connected.error().message << '\n'; return; }
    auto runtime = std::move(connected).value();
    std::mutex mutex;
    std::condition_variable cv;
    std::deque<fm::m::FileMetadataMT> messages;
    auto subscription = svc::exchange_input::subscribe(runtime, [&](const fm::m::FileMetadataMT& value) {
        std::lock_guard<std::mutex> lock(mutex);
        messages.push_back(value); // retain a checked owning copy beyond callback lifetime
        cv.notify_one();
    });
    CHECK(subscription.ok());
    if (!subscription.ok()) return;
    int sequence = 0;
    for (int payload : {0, 1, 0}) {
        const bool hash = include_hash && sequence++ != 2;
        {
            const auto value = fm::make(payload, hash);
            auto published = svc::exchange_output::publish(runtime, value);
            CHECK(published.ok());
            if (!published.ok()) std::cerr << published.error().message << '\n';
        } // publisher's payload destroyed before inspecting subscriber's copy
        std::unique_lock<std::mutex> lock(mutex);
        CHECK(cv.wait_for(lock, milliseconds(10000), [&] { return !messages.empty(); }));
        if (!messages.empty()) {
            CHECK(fm::verify(messages.front(), payload, hash));
            messages.pop_front();
        }
        lock.unlock();
        CHECK(!runtime.poll_event(milliseconds(0)).has_value());
    }
    CHECK(subscription->unsubscribe().ok());
    CHECK(svc::exchange_output::publish(runtime, fm::make(1, false)).ok());
    {
        std::unique_lock<std::mutex> lock(mutex);
        CHECK(!cv.wait_for(lock, milliseconds(300), [&] { return !messages.empty(); }));
    }
    CHECK_EQ(runtime.stats().delivered, 3U);
    CHECK_EQ(runtime.stats().decode_errors, 0U);
    CHECK_EQ(runtime.stats().handler_exceptions, 0U);
    CHECK_EQ(runtime.stats().unknown_subscription, 0U);
    CHECK(runtime.close().ok());
    CHECK(!runtime.connected());
    CHECK(check::eventually([] { return check::threads_named("ams-lacal") == 0; }));
}
TEST_CASE(real_file_metadata_without_hash) { roundtrip(false); }
// This MUST fail while the pinned broker counts lexical digits instead of
// octets. Kept as an executable acceptance test, never an expected-pass claim.
TEST_CASE(real_file_metadata_with_hash) { roundtrip(true); }
int main(int argc, char** argv) {
    return check::run_all(("CPP_OMS_007_REAL_FILE_METADATA uci=" + std::to_string(UCI_FILE_METADATA_VERSION)).c_str(), argc, argv);
}