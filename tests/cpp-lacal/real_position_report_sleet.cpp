// CPP-OMS-004 Stage E: authentic UCI PositionReport publish/subscribe through
// the C++ LA-CAL runtime (cpp/oms-lacal-runtime, unchanged) and the
// UNMODIFIED pinned Sleet broker, with Float64 values on the wire.
//
//   -DUCI_PR_VERSION=25 | 26, generated headers from
//   tests/fixtures/service-generate/position-report-loop.yaml, -I tests/cpp-codec
//   CPP_OMS_004_SLEET_URL=ws://127.0.0.1:PORT CPP_OMS_004_GOLDEN=<golden> real_position_report_sleet
//
// Run by scripts/check-cpp-oms-004-position-report.sh, which starts Sleet with
// the matching full pinned schema. Application traffic goes ONLY through the
// generated facade (service_api.hpp) and the generated codec
// (service_codec.hpp). The independent raw client (raw_owp_client.hpp)
// observes the actual MSG text and publishes documents the runtime never
// produced; every expected document and bit pattern comes from
// position_report_documents.hpp (hand-written, oracle-checked).
#include <unistd.h>

#include <condition_variable>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <deque>
#include <fstream>
#include <iostream>
#include <mutex>
#include <optional>
#include <set>
#include <string>
#include <thread>
#include <vector>

#include "ams_gra/lacal/runtime.hpp"
#include "check.hpp"
#include "oam.hpp"
#include "position_report_documents.hpp"
#include "raw_owp_client.hpp"
#include "service_api.hpp"
#include "service_codec.hpp"

AMS_GRA_LACAL_PAYLOAD_CODEC(::programs::oam::PositionReportMT, ::service_codec::encode_PositionReportMT,
                            ::service_codec::decode_PositionReportMT)

namespace m = ::programs::oam;
namespace sc = ::service_codec;
namespace lacal = ams_gra::lacal;
namespace svc = ::service_api::function_loop;
using std::chrono::milliseconds;

namespace {

const std::string kService = "position-report-loop";
const std::string kTopic = "mission.position-report";
const milliseconds kWait(10000);
const milliseconds kQuiet(300);

std::string env(const char* name) {
    const char* value = std::getenv(name);
    if (value == nullptr || *value == '\0') {
        std::cerr << name << " is required\n";
        std::exit(2);
    }
    return value;
}

std::string read_file(const std::string& path) {
    std::FILE* file = std::fopen(path.c_str(), "rb");
    if (file == nullptr) {
        std::cerr << "cannot read " << path << '\n';
        std::exit(2);
    }
    std::string text;
    char chunk[4096];
    std::size_t n = 0;
    while ((n = std::fread(chunk, 1, sizeof(chunk), file)) > 0) {
        text.append(chunk, n);
    }
    std::fclose(file);
    return text;
}

m::PositionReportMT decode_ok(const std::string& json) {
    auto result = sc::decode_PositionReportMT(json);
    if (!result.ok()) {
        std::cerr << "test setup: decode failed at " << result.error().path << ": " << result.error().message << '\n';
        std::exit(2);
    }
    return std::move(result).value();
}

std::string encode(const m::PositionReportMT& value) {
    auto result = sc::encode_PositionReportMT(value);
    return result.ok() ? result.value() : "<encode failed: " + result.error().message + ">";
}

void no_failures(const std::vector<std::string>& failures, const char* what) {
    for (const std::string& failure : failures) {
        std::cerr << what << ": " << failure << '\n';
    }
    CHECK(failures.empty());
}

std::string envelope(const std::string& body) { return "{\"PositionReport\":" + body + "}"; }

/// Typed deliveries from the generated subscription.
struct Inbox {
    std::mutex mutex;
    std::condition_variable cv;
    std::deque<m::PositionReportMT> values;
    void add(const m::PositionReportMT& value) {
        {
            std::lock_guard<std::mutex> lock(mutex);
            values.push_back(value);
        }
        cv.notify_all();
    }
    std::optional<m::PositionReportMT> take(milliseconds wait) {
        std::unique_lock<std::mutex> lock(mutex);
        if (!cv.wait_for(lock, wait, [&] { return !values.empty(); })) {
            return std::nullopt;
        }
        m::PositionReportMT value = std::move(values.front());
        values.pop_front();
        return value;
    }
};

struct Url {
    std::string host;
    std::string port;
};

Url split(const std::string& url) {
    const std::string rest = url.substr(url.find("://") + 3);
    return Url{rest.substr(0, rest.rfind(':')), rest.substr(rest.rfind(':') + 1)};
}

/// Established TCP connections from this process to the broker port.
int connections_to(const std::string& port) {
    std::set<std::string> inodes;
    for (int fd = 0; fd < 1024; ++fd) {
        char link[256] = {};
        const std::string path = "/proc/self/fd/" + std::to_string(fd);
        const ssize_t n = readlink(path.c_str(), link, sizeof(link) - 1);
        if (n > 0) {
            const std::string target(link, static_cast<std::size_t>(n));
            if (target.rfind("socket:[", 0) == 0) {
                inodes.insert(target.substr(8, target.size() - 9));
            }
        }
    }
    char hex[8];
    std::snprintf(hex, sizeof(hex), "%04X", static_cast<unsigned>(std::stoul(port)));
    std::ifstream tcp("/proc/net/tcp");
    std::string line;
    std::getline(tcp, line);
    int count = 0;
    while (std::getline(tcp, line)) {
        std::vector<std::string> fields;
        std::size_t at = 0;
        while (at < line.size()) {
            const std::size_t start = line.find_first_not_of(' ', at);
            if (start == std::string::npos) {
                break;
            }
            const std::size_t end = line.find(' ', start);
            fields.push_back(line.substr(start, end == std::string::npos ? std::string::npos : end - start));
            at = end == std::string::npos ? line.size() : end;
        }
        if (fields.size() < 10) {
            continue;
        }
        const std::string& remote = fields[2];
        if (remote.size() >= 4 && remote.substr(remote.size() - 4) == hex && fields[3] == "01" &&
            inodes.count(fields[9]) != 0) {
            ++count;
        }
    }
    return count;
}

lacal::Runtime connect_runtime(const std::string& url) {
    lacal::Config config;
    config.url = url;
    config.service_id = kService;
    config.schema_version = pr::kVersion;
    config.connect_timeout = kWait;
    config.operation_timeout = kWait;
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(60);
    for (;;) {
        auto runtime = lacal::Runtime::connect(config);
        if (runtime.ok()) {
            return std::move(runtime).value();
        }
        if (std::chrono::steady_clock::now() > deadline) {
            std::cerr << "Sleet connect/INIT failed: " << lacal::to_string(runtime.error().kind) << ": "
                      << runtime.error().message << '\n';
            std::exit(2);
        }
        std::this_thread::sleep_for(milliseconds(200));
    }
}

void say(const std::string& text) { std::cout << "CPP-OMS-004 " << pr::kVersion << " " << text << std::endl; }

}  // namespace

TEST_CASE(golden_and_generated_identity) {
    // The global wire element is PositionReport, never PositionReportMT.
    CHECK_EQ(std::string(svc::exchange_input_position::message_name), std::string("PositionReport"));
    CHECK_EQ(std::string(svc::exchange_output_position::message_name), std::string("PositionReport"));
    CHECK_EQ(std::string(svc::exchange_input_position::message_namespace), std::string(lacal::owp::oam_namespace));
    CHECK_EQ(std::string(svc::exchange_input_position::topic), kTopic);
    CHECK_EQ(std::string(svc::exchange_output_position::topic), kTopic);
    CHECK_EQ(lacal::owp::global_element_name(svc::exchange_output_position::message_namespace,
                                             svc::exchange_output_position::message_name),
             std::string("PositionReport"));
    const m::PositionReportMT golden = decode_ok(pr::payload_body(read_file(env("CPP_OMS_004_GOLDEN"))));
    no_failures(pr::verify_golden(golden), "golden");
    CHECK_EQ(encode(golden), pr::kGoldenCanonical);
    const m::PositionReportMT full = decode_ok(pr::kFull);
    no_failures(pr::verify_full(full), "full");
    CHECK_EQ(encode(full), pr::kFull);
    say("GOLDEN: PASSED");
}

TEST_CASE(pinned_sleet_float64_roundtrip_rejections_and_recovery) {
    const std::string url = env("CPP_OMS_004_SLEET_URL");
    const Url where = split(url);
    lacal::Runtime runtime = connect_runtime(url);
    CHECK(runtime.server_info().find("\"version\":\"1.0\"") != std::string::npos);

    Inbox inbox;
    auto subscription = svc::exchange_input_position::subscribe(
        runtime, [&](const svc::exchange_input_position::Payload& value) { inbox.add(value); });
    CHECK(subscription.ok());
    if (!subscription.ok()) {
        std::cerr << "subscribe: " << subscription.error().message << '\n';
        return;
    }

    raw::Client observer(where.host, where.port, kService, pr::kVersion, kWait);
    raw::Client publisher(where.host, where.port, kService, pr::kVersion, kWait);
    CHECK_EQ(observer.command("SUB wire PositionReport " + kTopic), std::string("+OK"));
    CHECK_EQ(connections_to(where.port), 3);

    const auto observe = [&]() -> std::string {
        auto frame = observer.receive();
        if (!frame || frame->rfind("MSG wire ", 0) != 0) {
            return "<no MSG>";
        }
        return frame->substr(9);
    };
    const auto no_event = [&] { return !runtime.poll_event(milliseconds(0)).has_value(); };

    // ---- 1. generated publish -> Sleet -> raw observer + typed callback ------
    struct Typed {
        const char* name;
        std::string canonical;
        bool specials;
        bool golden;
    };
    const std::vector<Typed> typed = {
        {"golden", pr::kGoldenCanonical, false, true},
        {"full Float64", pr::kFull, false, false},
        {"NaN/Infinity/-Infinity", pr::kSpecials, true, false},
    };
    for (const Typed& item : typed) {
        const m::PositionReportMT value = decode_ok(item.canonical);
        lacal::Status published = svc::exchange_output_position::publish(runtime, value);
        CHECK(published.ok());                                   // Sleet validated and acknowledged (+OK)
        CHECK_EQ(observe(), envelope(item.canonical));           // actual MSG bytes, byte for byte
        auto received = inbox.take(kWait);
        CHECK(received.has_value());
        if (received) {
            no_failures(item.golden ? pr::verify_golden(*received) : pr::verify_full(*received, item.specials),
                        item.name);
            CHECK_EQ(encode(*received), item.canonical);
        }
        CHECK(no_event());
        say(std::string("typed MSG ") + item.name + ": PASSED");
    }

    const m::PositionReportMT full = decode_ok(pr::kFull);
    const auto recover = [&](const std::string& after) {
        CHECK(svc::exchange_output_position::publish(runtime, full).ok());
        CHECK_EQ(observe(), envelope(pr::kFull));
        auto recovered = inbox.take(kWait);
        CHECK(recovered.has_value());
        if (recovered) {
            no_failures(pr::verify_full(*recovered), "recovery");
        }
        CHECK(no_event());
        say(after + ": same-subscription recovery PASSED");
    };

    // ---- 2. independently authored documents from the raw publisher ----------
    // The pinned golden text itself (2.5: the Sleet fixture envelope verbatim;
    // 2.6: the PY-016 body in an envelope), so the authentic broker validates
    // the release's input against the full schema.
    {
        const std::string text = read_file(env("CPP_OMS_004_GOLDEN"));
        const std::string document = pr::payload_body(text) == text ? envelope(text) : text;
        CHECK_EQ(publisher.command("PUB " + kTopic + " " + document), std::string("+OK"));
        CHECK_EQ(observe(), document);
        auto received = inbox.take(kWait);
        CHECK(received.has_value());
        if (received) {
            no_failures(pr::verify_golden(*received), "raw golden");
            CHECK_EQ(encode(*received), pr::kGoldenCanonical);
        }
        CHECK(no_event());
        say("raw pinned golden accepted by Sleet and decoded: PASSED");
    }
    // Non-canonical spellings of the same values: delivered verbatim, decoded
    // to the same bits (correct rounding, signed zero, underflow).
    {
        std::string loose = pr::with(pr::kFull, R"("Latitude":1.5707963267948966)", R"("Latitude":15707963267948966e-16)");
        loose = pr::with(loose, R"("HAE_Adjustment":-0)", R"("HAE_Adjustment":-0.0E+0)");
        loose = pr::with(loose, R"("SemiMinorAxisLength":5e-324)", R"("SemiMinorAxisLength":2.4703282292062328e-324)");
        loose = pr::with(loose, R"("UncertaintyVertical":0)", R"("UncertaintyVertical":1e-400)");
        loose = pr::with(loose, R"("Roll":0.1)", R"("Roll":0.1000000000000000055511151231257827)");
        loose = pr::with(loose, R"("SemiMajorAxisLength":1e+21)", R"("SemiMajorAxisLength":1000000000000000000000.0)");
        CHECK_EQ(publisher.command("PUB " + kTopic + " " + envelope(loose)), std::string("+OK"));
        CHECK_EQ(observe(), envelope(loose));
        auto received = inbox.take(kWait);
        CHECK(received.has_value());
        if (received) {
            no_failures(pr::verify_full(*received), "non-canonical");
            CHECK_EQ(encode(*received), pr::kFull);
        }
        CHECK(no_event());
        say("raw non-canonical Float64 spellings: PASSED");
    }

    // ---- 3. malformed documents ---------------------------------------------
    struct Bad {
        std::string name;
        std::string from;
        std::string to;
        bool sleet_delivers;     // the pinned Sleet admits it (no facet check on special strings)
        std::string diagnostic;  // codec path suffix (delivered) or -ERR fragment (rejected)
    };
    const std::string inertial = "PositionReportMT.MessageData.InertialState.";
    const std::vector<Bad> bad = {
        // Sleet delivers: the generated model's domain check rejects on the client.
        {"Latitude NaN", R"("Latitude":1.5707963267948966)", R"("Latitude":"NaN")", true, inertial + "Position.Latitude"},
        {"Latitude Infinity", R"("Latitude":1.5707963267948966)", R"("Latitude":"Infinity")", true,
         inertial + "Position.Latitude"},
        {"Altitude -Infinity", R"("Altitude":-6378237)", R"("Altitude":"-Infinity")", true,
         inertial + "Position.Altitude"},
        {"SemiMajorAxisLength -Infinity", R"("SemiMajorAxisLength":1e+21)", R"("SemiMajorAxisLength":"-Infinity")", true,
         inertial + "PositionUncertainty.UncertaintyEllipse.SemiMajorAxisLength"},
        {"WanderAngle Infinity", R"("WanderAngle":0.9999999999999999)", R"("WanderAngle":"Infinity")", true,
         "PositionReportMT.MessageData.WanderAngle"},
        // Sleet rejects: -ERR to the publisher, nothing delivered.
        {"Latitude one ulp above pi/2", R"("Latitude":1.5707963267948966)", R"("Latitude":1.5707963267948968)", false,
         "maxInclusive"},
        {"Longitude one ulp below -pi", R"("Longitude":-3.141592653589793)", R"("Longitude":-3.1415926535897936)", false,
         "minInclusive"},
        {"Altitude below the bound", R"("Altitude":-6378237)", R"("Altitude":-6378237.000000001)", false,
         "minInclusive"},
        {"negative subnormal distance", R"("SemiMinorAxisLength":5e-324)", R"("SemiMinorAxisLength":-5e-324)", false,
         "SemiMinorAxisLength"},
        {"lowercase nan", R"("NorthSpeed":2.2250738585072014e-308)", R"("NorthSpeed":"nan")", false,
         "number-or-special-float-string"},
        {"quoted number", R"("NorthSpeed":2.2250738585072014e-308)", R"("NorthSpeed":"1.5")", false,
         "number-or-special-float-string"},
        {"boolean", R"("NorthSpeed":2.2250738585072014e-308)", R"("NorthSpeed":true)", false,
         "number-or-special-float-string"},
        {"null", R"("NorthSpeed":2.2250738585072014e-308)", R"("NorthSpeed":null)", false,
         "number-or-special-float-string"},
        {"decimal overflow", R"("NorthAcceleration":9.80665)", R"("NorthAcceleration":1e309)", false,
         "number out of range"},
        {"missing Latitude", R"("Latitude":1.5707963267948966,)", "", false, "Latitude"},
    };
    std::uint64_t decode_errors = 0;
    for (const Bad& item : bad) {
        const std::string document = envelope(pr::with(pr::kFull, item.from, item.to));
        const std::string status = publisher.command("PUB " + kTopic + " " + document);
        if (item.sleet_delivers) {
            CHECK_EQ(status, std::string("+OK"));
            CHECK_EQ(observe(), document);  // the MSG really crossed Sleet
            auto event = runtime.poll_event(kWait);
            CHECK(event.has_value() && event->kind == lacal::EventKind::decode_error);
            if (event) {
                ++decode_errors;
                CHECK(event->error.kind == lacal::ErrorKind::codec);  // subscription decode error, not broker
                CHECK_EQ(event->message_name, std::string("PositionReport"));
                CHECK_EQ(event->sid, subscription->id());
                CHECK_EQ(event->error.codec_path, item.diagnostic);
                CHECK(event->error.message.find("domain check") != std::string::npos);
                say(item.name + ": Sleet MSG -> generated decoder rejected at " + event->error.codec_path + ": " +
                    event->error.message);
            }
        } else {
            CHECK(status.rfind("-ERR Invalid-Message", 0) == 0);
            CHECK(status.find(item.diagnostic) != std::string::npos);
            CHECK(!observer.receive(kQuiet).has_value());
            CHECK(!runtime.poll_event(kQuiet).has_value());
            say(item.name + ": Sleet rejected: " + status.substr(0, 150));
        }
        CHECK(!inbox.take(kQuiet).has_value());  // never a typed delivery
        recover(item.name);
    }
    {
        const std::string status = publisher.command("PUB " + kTopic + " {\"PositionReportMT\":" + pr::kFull + "}");
        CHECK(status.rfind("-ERR Invalid-Message", 0) == 0 && status.find("PositionReportMT") != std::string::npos);
        CHECK(!observer.receive(kQuiet).has_value());
        say("wrong global element PositionReportMT: Sleet rejected: " + status.substr(0, 120));
        recover("wrong global element");
    }

    // ---- 4. runtime-level broker rejection vs local rejection ---------------
    {
        lacal::Status denied = runtime.publish(svc::exchange_output_position::message_namespace,
                                               svc::exchange_output_position::message_name, "unauthorized-topic", full);
        CHECK(denied.error().kind == lacal::ErrorKind::broker_rejected);
        CHECK_EQ(denied.error().owp_code, std::string("Illegal-State"));
        say("runtime PUB unauthorized topic: broker_rejected -ERR " + denied.error().owp_code + " " +
            denied.error().owp_details);
        lacal::Status invalid_topic = runtime.publish(svc::exchange_output_position::message_namespace,
                                                      svc::exchange_output_position::message_name, "invalid topic", full);
        CHECK(invalid_topic.error().kind == lacal::ErrorKind::invalid_argument);
        // Local codec error: a Latitude carrier whose bytes were forged
        // outside its domain is refused by the generated encoder; nothing sent.
        m::PositionReportMT forged = full;
        const double outside = 1.5707963267948968;
        std::memcpy(static_cast<void*>(&forged.messagedata.inertialstate.position.latitude), &outside, sizeof outside);
        lacal::Status codec = svc::exchange_output_position::publish(runtime, forged);
        CHECK(codec.error().kind == lacal::ErrorKind::codec);
        CHECK_EQ(codec.error().codec_path, inertial + "Position.Latitude");
        say("local codec rejection at " + codec.error().codec_path + ": " + codec.error().message + " (nothing sent)");
        CHECK(!observer.receive(kQuiet).has_value());
        CHECK(runtime.connected());
        recover("runtime-level rejections");
    }

    // ---- 5. UNSUB and cleanup -------------------------------------------------
    CHECK(subscription->unsubscribe().ok());  // verbose: Sleet acknowledged UNSUB
    CHECK(svc::exchange_output_position::publish(runtime, full).ok());
    CHECK_EQ(observe(), envelope(pr::kFull));
    CHECK(!inbox.take(kQuiet).has_value());  // no callback after UNSUB
    CHECK(no_event());
    CHECK(runtime.close().ok());
    CHECK(check::threads_named("ams-lacal") == 0);
    CHECK(check::eventually([&] { return connections_to(where.port) == 2; }));  // only the raw clients remain
    CHECK_EQ(observer.command("UNSUB wire"), std::string("+OK"));
    const lacal::Stats stats = runtime.stats();
    // 3 typed + 2 raw + one recovery per malformed case (15 + wrong element) + runtime-level.
    CHECK_EQ(stats.delivered, 3U + 2U + bad.size() + 1U + 1U);
    CHECK_EQ(stats.decode_errors, decode_errors);
    CHECK_EQ(decode_errors, 5U);
    CHECK_EQ(stats.unknown_subscription, 0U);
    CHECK_EQ(stats.handler_exceptions, 0U);
    say("SLEET AND RECOVERY: PASSED");
}

int main(int argc, char** argv) {
    return check::run_all(("CPP_OMS_004_REAL_POSITION_REPORT_COMPLETE uci=" + std::to_string(UCI_PR_VERSION)).c_str(),
                          argc, argv);
}
