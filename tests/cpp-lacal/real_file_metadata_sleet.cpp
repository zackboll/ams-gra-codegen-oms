// Authentic CPP-OMS-007 traffic through the existing LA-CAL runtime and Sleet.
#include "ams_gra/lacal/runtime.hpp"
#include "check.hpp"
#include "file_metadata_documents.hpp"
#include "service_api.hpp"
#include "raw_owp_client.hpp"
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
    // A separate publisher bypasses the generated codec to exercise broker
    // validation, rather than proving only local checked-carrier rejection.
    const std::string address(url);
    const auto colon = address.rfind(':');
    raw::Client publisher("127.0.0.1", address.substr(colon + 1), "cpp-oms-007", fm::version,
                          milliseconds(10000));
    auto take = [&](int payload, bool hash) {
        std::unique_lock<std::mutex> lock(mutex);
        CHECK(cv.wait_for(lock, milliseconds(10000), [&] { return !messages.empty(); }));
        if (!messages.empty()) {
            CHECK(fm::verify(messages.front(), payload, hash));
            messages.pop_front();
        }
        lock.unlock();
        CHECK(!runtime.poll_event(milliseconds(0)).has_value());
    };
    int sequence = 0;
    for (int payload : {0, 1, 0}) {
        const bool hash = include_hash && sequence++ != 2;
        {
            const auto value = fm::make(payload, hash);
            auto published = svc::exchange_output::publish(runtime, value);
            CHECK(published.ok());
            if (!published.ok()) std::cerr << published.error().message << '\n';
        } // publisher's payload destroyed before inspecting subscriber's copy
        take(payload, hash);
    }
    for (const std::string& invalid : {std::string(62, '0'), std::string(66, 'F'),
                                      std::string(63, '0'), std::string(64, 'G'), std::string("00 00")}) {
        std::string body = fm::document(0, true);
        const auto offset = body.find(std::string(64, '0'));
        CHECK(offset != std::string::npos);
        body.replace(offset, 64, invalid);
        const auto status = publisher.command("PUB file.metadata {\"FileMetadata\":" + body + "}");
        CHECK(status.rfind("-ERR Invalid-Message", 0) == 0);
        CHECK(status.find("FileMetadata.MessageData.SHA_2_Hash") != std::string::npos);
        CHECK(status.find(invalid.size() == 62 || invalid.size() == 66 ? "length" : "hexBinary") != std::string::npos);
        {
            std::unique_lock<std::mutex> lock(mutex);
            CHECK(!cv.wait_for(lock, milliseconds(100), [&] { return !messages.empty(); }));
        }
        CHECK(!runtime.poll_event(milliseconds(0)).has_value());
        // Same-subscription typed recovery, using a valid full-size hash.
        CHECK(svc::exchange_output::publish(runtime, fm::make(1, include_hash)).ok());
        take(1, include_hash);
    }
    CHECK(subscription->unsubscribe().ok());
    CHECK(svc::exchange_output::publish(runtime, fm::make(1, false)).ok());
    {
        std::unique_lock<std::mutex> lock(mutex);
        CHECK(!cv.wait_for(lock, milliseconds(300), [&] { return !messages.empty(); }));
    }
    CHECK_EQ(runtime.stats().delivered, 8U);
    CHECK_EQ(runtime.stats().decode_errors, 0U);
    CHECK_EQ(runtime.stats().handler_exceptions, 0U);
    CHECK_EQ(runtime.stats().unknown_subscription, 0U);
    CHECK(runtime.close().ok());
    CHECK(!runtime.connected());
    CHECK(check::eventually([] { return check::threads_named("ams-lacal") == 0; }));
}
TEST_CASE(real_file_metadata_without_hash) { roundtrip(false); }
// This MUST fail with the original pin, and pass with an approved correction.
TEST_CASE(real_file_metadata_with_hash) { roundtrip(true); }
int main(int argc, char** argv) {
    return check::run_all(("CPP_OMS_007_REAL_FILE_METADATA uci=" + std::to_string(UCI_FILE_METADATA_VERSION)).c_str(), argc, argv);
}