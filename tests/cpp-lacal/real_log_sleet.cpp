// CPP-OMS-003 Stage C: authentic UCI `Log` publish/subscribe through the C++
// LA-CAL runtime and the UNMODIFIED pinned Sleet broker.
//
//   -DUCI_LOG_VERSION=25 | 26, generated headers from tests/fixtures/task102
//   CPP_OMS_003_SLEET_URL=ws://127.0.0.1:PORT CPP_OMS_003_GOLDEN=<task093 golden> real_log_sleet
//
// Run by scripts/check-cpp-oms-003-real-log-sleet.sh, which starts Sleet with
// the matching full pinned schema. Application traffic goes ONLY through the
// generated facade (service_api.hpp) and the generated codec
// (service_codec.hpp). The independent raw client (raw_owp_client.hpp)
// observes the actual MSG text and publishes malformed documents; every
// expected JSON document here is written by hand, never produced by the
// encoder under test.
#include <unistd.h>

#include <condition_variable>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <deque>
#include <fstream>
#include <iostream>
#include <mutex>
#include <optional>
#include <set>
#include <string>
#include <thread>
#include <type_traits>
#include <variant>
#include <vector>

#include "ams_gra/lacal/runtime.hpp"
#include "check.hpp"
#include "oam.hpp"
#include "raw_owp_client.hpp"
#include "service_api.hpp"
#include "service_codec.hpp"

#ifndef UCI_LOG_VERSION
#error "define UCI_LOG_VERSION=25 or 26"
#endif

AMS_GRA_LACAL_PAYLOAD_CODEC(::programs::oam::LogMT, ::service_codec::encode_LogMT, ::service_codec::decode_LogMT)

namespace m = ::programs::oam;
namespace sc = ::service_codec;
namespace lacal = ams_gra::lacal;
namespace svc = ::service_api::function_log;
using std::chrono::milliseconds;

namespace {

#if UCI_LOG_VERSION == 25
const std::string kVersion = "002.5.0";
#elif UCI_LOG_VERSION == 26
const std::string kVersion = "002.6.0";
#else
#error "UCI_LOG_VERSION must be 25 or 26"
#endif
const std::string kTopic = "task102-log";
const milliseconds kWait(10000);
const milliseconds kQuiet(300);

// ---- independently authored documents (compact, schema order) --------------
const std::string kSecurity =
    R"("SecurityInformation":{"Classification":"U","OwnerProducer":[{"GovernmentIdentifier":"USA"},{"NATO_SpecialWord":"NATO:EXAMPLE"}]})";
const std::string kHeader = R"("MessageHeader":{"SystemID":{"UUID":"12345678-1234-1234-8234-123456789abc"},"Timestamp":"2026-01-02T03:04:05Z","SchemaVersion":")" +
                            kVersion + R"(","Mode":"LIVE"})";
const std::string kDataHead =
    R"("MessageData":{"Timestamp":"2026-01-02T03:04:06Z","ServiceUpTime":"PT01.2300S","Severity":"INFO","Label":"Task093","Details":"Independent real Log Binary codec qualification")";

std::string document(const std::string& data_member) {
    return "{" + kSecurity + "," + kHeader + "," + kDataHead + data_member + "}}";
}
const std::string kGolden = document(R"(,"Data":"0080FF")");

std::string with(const std::string& text, const std::string& from, const std::string& to) {
    const std::size_t at = text.find(from);
    if (at == std::string::npos || text.find(from, at + 1) != std::string::npos) {
        std::cerr << "test setup: '" << from << "' must occur exactly once\n";
        std::exit(2);
    }
    std::string result = text;
    result.replace(at, from.size(), to);
    return result;
}

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

// ---- model helpers (release differences: carrier of MessageData.Data) -------
using DataCarrier = typename decltype(m::LogMDT::data)::value_type;

template <typename D>
D make_carrier(std::vector<std::uint8_t> octets) {
    if constexpr (std::is_same_v<D, std::vector<std::uint8_t>>) {
        return octets;
    } else {
        return D(std::move(octets));
    }
}

template <typename D>
const std::vector<std::uint8_t>& octets_of(const D& data) {
    if constexpr (std::is_same_v<D, std::vector<std::uint8_t>>) {
        return data;
    } else {
        return data.value();
    }
}

DataCarrier make_data(std::vector<std::uint8_t> octets) { return make_carrier<DataCarrier>(std::move(octets)); }

std::optional<std::vector<std::uint8_t>> data_of(const m::LogMT& log) {
    if (!log.messagedata.data) {
        return std::nullopt;
    }
    return octets_of(*log.messagedata.data);
}

m::LogMT decode_ok(const std::string& json) {
    auto result = sc::decode_LogMT(json);
    if (!result.ok()) {
        std::cerr << "test setup: decode failed at " << result.error().path << ": " << result.error().message << '\n';
        std::exit(2);
    }
    return std::move(result).value();
}

std::string encode(const m::LogMT& value) {
    auto result = sc::encode_LogMT(value);
    return result.ok() ? result.value() : "<encode failed: " + result.error().message + ">";
}

/// Typed inspection of the fields every case shares (independent of the codec).
void inspect(const m::LogMT& log) {
    const auto& security = log.securityinformation;
    CHECK(security.classification == m::ClassificationEnum::U);
    CHECK_EQ(security.ownerproducer.values().size(), 2U);
    if (security.ownerproducer.values().size() == 2) {
        const auto* government =
            std::get_if<m::OwnerProducerChoiceType::GovernmentIdentifier>(&security.ownerproducer.values()[0].value);
        CHECK(government != nullptr && government->value == m::OwnerProducerEnum::USA);
        const auto* nato =
            std::get_if<m::OwnerProducerChoiceType::NATOSpecialWord>(&security.ownerproducer.values()[1].value);
        CHECK(nato != nullptr && nato->value.value() == "NATO:EXAMPLE");
    }
    CHECK_EQ(log.messageheader.systemid.uuid.value(), std::string("12345678-1234-1234-8234-123456789abc"));
    CHECK_EQ(log.messageheader.timestamp.value(), std::string("2026-01-02T03:04:05Z"));
    CHECK_EQ(log.messageheader.schemaversion.value(), kVersion);
    CHECK(log.messageheader.mode == m::MessageModeEnum::LIVE);
    CHECK_EQ(log.messagedata.timestamp.value(), std::string("2026-01-02T03:04:06Z"));
    CHECK(log.messagedata.serviceuptime.has_value() && log.messagedata.serviceuptime->value() == "PT01.2300S");
    CHECK(log.messagedata.severity == m::LogSeverityEnum::INFO);
    CHECK_EQ(log.messagedata.label.value(), std::string("Task093"));
    CHECK_EQ(log.messagedata.details.value(), std::string("Independent real Log Binary codec qualification"));
}

/// Typed deliveries from the generated subscription.
struct Inbox {
    std::mutex mutex;
    std::condition_variable cv;
    std::deque<m::LogMT> values;
    void add(const m::LogMT& value) {
        {
            std::lock_guard<std::mutex> lock(mutex);
            values.push_back(value);
        }
        cv.notify_all();
    }
    std::optional<m::LogMT> take(milliseconds wait) {
        std::unique_lock<std::mutex> lock(mutex);
        if (!cv.wait_for(lock, wait, [&] { return !values.empty(); })) {
            return std::nullopt;
        }
        m::LogMT value = std::move(values.front());
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
        std::vector<std::string> fields;  // whitespace-separated /proc/net/tcp columns
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
    config.service_id = kTopic;
    config.schema_version = kVersion;
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

}  // namespace

TEST_CASE(golden_and_generated_identity) {
    CHECK_EQ(std::string(svc::exchange_receive::message_name), std::string("Log"));  // not LogMT
    CHECK_EQ(std::string(svc::exchange_publish::message_name), std::string("Log"));
    CHECK_EQ(std::string(svc::exchange_receive::message_namespace), std::string(lacal::owp::oam_namespace));
    CHECK_EQ(std::string(svc::exchange_receive::topic), kTopic);
    CHECK_EQ(std::string(svc::exchange_publish::topic), kTopic);
    CHECK_EQ(lacal::owp::global_element_name(svc::exchange_publish::message_namespace,
                                             svc::exchange_publish::message_name),
             std::string("Log"));
    const m::LogMT golden = decode_ok(read_file(env("CPP_OMS_003_GOLDEN")));
    inspect(golden);
    CHECK(data_of(golden) == std::vector<std::uint8_t>({0x00, 0x80, 0xFF}));
    CHECK_EQ(encode(golden), kGolden);
    std::cout << "CPP-OMS-003 " << kVersion << " GOLDEN: PASSED" << std::endl;
}

TEST_CASE(pinned_sleet_roundtrip_rejections_and_recovery) {
    const std::string url = env("CPP_OMS_003_SLEET_URL");
    const Url where = split(url);
    lacal::Runtime runtime = connect_runtime(url);
    CHECK(runtime.server_info().find("\"version\":\"1.0\"") != std::string::npos);

    Inbox inbox;
    auto subscription = svc::exchange_receive::subscribe(runtime, [&](const svc::exchange_receive::Payload& value) {
        inbox.add(value);
    });
    CHECK(subscription.ok());
    if (!subscription.ok()) {
        std::cerr << "subscribe: " << subscription.error().message << '\n';
        return;
    }

    raw::Client observer(where.host, where.port, kTopic, kVersion, kWait);
    raw::Client publisher(where.host, where.port, kTopic, kVersion, kWait);
    CHECK_EQ(observer.command("SUB wire Log " + kTopic), std::string("+OK"));
    CHECK_EQ(connections_to(where.port), 3);

    const auto observe = [&]() -> std::string {
        auto frame = observer.receive();
        if (!frame || frame->rfind("MSG wire ", 0) != 0) {
            return "<no MSG>";
        }
        return frame->substr(9);
    };
    const auto no_event = [&] { return !runtime.poll_event(milliseconds(0)).has_value(); };

    // ---- 1. three binary states, generated publish -> Sleet -> typed callback
    m::LogMT value = decode_ok(kGolden);
    struct State {
        const char* name;
        std::optional<std::vector<std::uint8_t>> data;
        std::string member;
    };
    const std::vector<State> states = {
        {"absent", std::nullopt, ""},
        {"empty", std::vector<std::uint8_t>{}, R"(,"Data":"")"},
        {"0080FF", std::vector<std::uint8_t>{0x00, 0x80, 0xFF}, R"(,"Data":"0080FF")"},
    };
    for (const State& state : states) {
        value.messagedata.data = state.data ? std::optional<DataCarrier>(make_data(*state.data)) : std::nullopt;
        const std::string expected = document(state.member);
        lacal::Status published = svc::exchange_publish::publish(runtime, value);
        CHECK(published.ok());  // verbose: Sleet acknowledged the PUB with +OK
        CHECK_EQ(observe(), "{\"Log\":" + expected + "}");  // actual MSG bytes
        auto received = inbox.take(kWait);
        CHECK(received.has_value());
        if (received) {
            inspect(*received);
            CHECK(data_of(*received) == state.data);
            CHECK_EQ(encode(*received), expected);
        }
        CHECK(no_event());
        std::cout << "CPP-OMS-003 " << kVersion << " typed MSG Data=" << state.name << ": PASSED" << std::endl;
    }
    value.messagedata.data = make_data({0x00, 0x80, 0xFF});

    const auto recover = [&](const std::string& after) {
        CHECK(svc::exchange_publish::publish(runtime, value).ok());
        CHECK_EQ(observe(), "{\"Log\":" + kGolden + "}");
        auto recovered = inbox.take(kWait);
        CHECK(recovered.has_value());
        if (recovered) {
            inspect(*recovered);
            CHECK_EQ(encode(*recovered), kGolden);
        }
        CHECK(no_event());
        std::cout << "CPP-OMS-003 " << kVersion << " " << after << ": same-subscription recovery PASSED" << std::endl;
    };

    // ---- 2. malformed documents from the independent publisher -------------
    struct Bad {
        std::string name;
        std::string document;  // full global-element document
        bool sleet_delivers;   // pinned Sleet admits it (weak hexBinary lexical check)
        std::string diagnostic;
    };
    const auto data_case = [&](const char* name, const char* data, bool delivered) {
        return Bad{name, "{\"Log\":" + with(kGolden, R"("0080FF")", data) + "}", delivered, "Data"};
    };
    std::vector<Bad> bad = {
        data_case("odd hex", R"("0")", true),
        data_case("invalid digits", R"("0G")", true),
        data_case("internal whitespace", R"("00 80 FF")", true),
        data_case("binary JSON type", "[0,128,255]", false),
        {"missing Timestamp", "{\"Log\":" + with(kGolden, R"("Timestamp":"2026-01-02T03:04:06Z",)", "") + "}", false, "Timestamp"},
        {"missing Severity", "{\"Log\":" + with(kGolden, R"("Severity":"INFO",)", "") + "}", false, "Severity"},
        {"missing Label", "{\"Log\":" + with(kGolden, R"("Label":"Task093",)", "") + "}", false, "Label"},
        {"missing Details",
         "{\"Log\":" + with(kGolden, R"("Details":"Independent real Log Binary codec qualification",)", "") + "}", false,
         "Details"},
        {"missing MessageHeader", "{\"Log\":" + with(kGolden, kHeader + ",", "") + "}", false, "MessageHeader"},
        {"missing SecurityInformation", "{\"Log\":" + with(kGolden, kSecurity + ",", "") + "}", false,
         "SecurityInformation"},
        {"missing MessageData", "{\"Log\":" + with(kGolden, "," + kDataHead + R"(,"Data":"0080FF"})", "") + "}", false,
         "MessageData"},
        {"wrong global element", "{\"LogMT\":" + kGolden + "}", false, "LogMT"},
    };
    for (const Bad& item : bad) {
        const std::string status = publisher.command("PUB " + kTopic + " " + item.document);
        if (item.sleet_delivers) {
            CHECK_EQ(status, std::string("+OK"));
            CHECK_EQ(observe(), item.document);  // the malformed MSG really crossed Sleet
            auto event = runtime.poll_event(kWait);
            CHECK(event.has_value() && event->kind == lacal::EventKind::decode_error);
            if (event) {
                CHECK(event->error.kind == lacal::ErrorKind::codec);  // client-side codec rejection
                CHECK_EQ(event->message_name, std::string("Log"));
                CHECK_EQ(event->sid, subscription->id());
                CHECK(event->error.codec_path.find(item.diagnostic) != std::string::npos);
                std::cout << "CPP-OMS-003 " << kVersion << " " << item.name
                          << ": Sleet MSG -> generated decoder rejected at " << event->error.codec_path << ": "
                          << event->error.message << std::endl;
            }
        } else {
            // Broker rejection: -ERR to the publisher, no MSG, no event, no callback.
            CHECK(status.rfind("-ERR Invalid-Message", 0) == 0);
            CHECK(status.find(item.diagnostic) != std::string::npos);
            CHECK(!observer.receive(kQuiet).has_value());
            CHECK(!runtime.poll_event(kQuiet).has_value());
            std::cout << "CPP-OMS-003 " << kVersion << " " << item.name << ": Sleet rejected: " << status.substr(0, 160)
                      << std::endl;
        }
        CHECK(!inbox.take(kQuiet).has_value());  // never a typed delivery
        recover(item.name);
    }

    // ---- 3. runtime-level broker rejection vs local rejection ---------------
    {
        // Same generated message identity, an unauthorized topic: Sleet -ERR.
        lacal::Status denied = runtime.publish(svc::exchange_publish::message_namespace,
                                               svc::exchange_publish::message_name, "unauthorized-topic", value);
        CHECK(denied.error().kind == lacal::ErrorKind::broker_rejected);
        CHECK_EQ(denied.error().owp_code, std::string("Illegal-State"));
        CHECK(denied.error().owp_details.find("unauthorized-topic") != std::string::npos);
        std::cout << "CPP-OMS-003 " << kVersion << " runtime PUB unauthorized topic: broker_rejected -ERR "
                  << denied.error().owp_code << " " << denied.error().owp_details << std::endl;
        auto denied_sub = runtime.subscribe<m::LogMT>(svc::exchange_receive::message_namespace,
                                                      svc::exchange_receive::message_name, "unauthorized-topic",
                                                      std::nullopt, [](const m::LogMT&) {});
        CHECK(!denied_sub.ok() && denied_sub.error().kind == lacal::ErrorKind::broker_rejected);
        CHECK(!denied_sub.ok() && denied_sub.error().owp_code == "Illegal-State");
        std::cout << "CPP-OMS-003 " << kVersion << " runtime SUB unauthorized topic: broker_rejected -ERR "
                  << denied_sub.error().owp_code << " " << denied_sub.error().owp_details << std::endl;
        // Local rejections send nothing: an invalid OWP topic and a value the
        // generated encoder refuses (out-of-range enumerator).
        lacal::Status invalid_topic = runtime.publish(svc::exchange_publish::message_namespace,
                                                      svc::exchange_publish::message_name, "invalid topic", value);
        CHECK(invalid_topic.error().kind == lacal::ErrorKind::invalid_argument);
        m::LogMT unencodable = value;
        unencodable.messagedata.severity = static_cast<m::LogSeverityEnum>(250);
        lacal::Status codec = svc::exchange_publish::publish(runtime, unencodable);
        CHECK(codec.error().kind == lacal::ErrorKind::codec);
        CHECK(codec.error().codec_path.find("Severity") != std::string::npos);
        std::cout << "CPP-OMS-003 " << kVersion << " local codec rejection at " << codec.error().codec_path << ": "
                  << codec.error().message << " (nothing sent)" << std::endl;
        CHECK(!observer.receive(kQuiet).has_value());
        CHECK(runtime.connected());
        recover("runtime-level rejections");
    }

    // ---- 4. UNSUB and cleanup -------------------------------------------------
    CHECK(subscription->unsubscribe().ok());  // verbose: Sleet acknowledged UNSUB
    CHECK(svc::exchange_publish::publish(runtime, value).ok());
    CHECK_EQ(observe(), "{\"Log\":" + kGolden + "}");
    CHECK(!inbox.take(kQuiet).has_value());  // no callback after UNSUB
    CHECK(no_event());
    CHECK(runtime.close().ok());
    CHECK(check::threads_named("ams-lacal") == 0);
    CHECK(check::eventually([&] { return connections_to(where.port) == 2; }));  // only the raw clients remain
    CHECK_EQ(observer.command("UNSUB wire"), std::string("+OK"));
    const lacal::Stats stats = runtime.stats();
    CHECK_EQ(stats.delivered, 3U + 13U);  // three states + one recovery per case and after rejections
    CHECK_EQ(stats.decode_errors, 3U);
    CHECK_EQ(stats.unknown_subscription, 0U);
    std::cout << "CPP-OMS-003 " << kVersion << " SLEET AND RECOVERY: PASSED" << std::endl;
}

int main(int argc, char** argv) {
    return check::run_all(("CPP_OMS_003_REAL_LOG_COMPLETE uci=" + std::to_string(UCI_LOG_VERSION)).c_str(), argc, argv);
}
