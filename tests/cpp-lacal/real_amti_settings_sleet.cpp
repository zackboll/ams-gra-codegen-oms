// CPP-OMS-005 Stage E: authentic UCI AMTI_SettingsCommand publish/subscribe
// through the C++ LA-CAL runtime (cpp/oms-lacal-runtime, unchanged) and the
// UNMODIFIED pinned Sleet broker, with the zero-length EmptyType UnassignAll on
// the wire.
//
//   -DUCI_AMTI_VERSION=25 | 26, generated headers from
//   tests/fixtures/service-generate/amti-settings-loop.yaml, -I tests/cpp-codec
//   CPP_OMS_005_SLEET_URL=ws://127.0.0.1:PORT real_amti_settings_sleet
//
// Run by scripts/check-cpp-oms-005-amti-settings.sh, which starts Sleet with
// the matching full pinned schema. Application traffic goes ONLY through the
// generated facade (service_api.hpp) and the generated codec
// (service_codec.hpp, bound below). The independent raw client
// (raw_owp_client.hpp) observes the actual MSG bytes and publishes documents
// the runtime never produced; every expected document comes from
// amti_settings_documents.hpp (hand-written).
//
// Two different rejection authorities are exercised and kept apart:
//   * Sleet schema rejection: the broker validates the published document
//     against the pinned XSD and answers -ERR Invalid-Message; nothing is
//     delivered, no client event, no callback.
//   * Generated C++ decoder rejection: Sleet accepts and forwards the bytes
//     verbatim, and the subscriber's generated codec refuses them, surfacing a
//     subscription decode_error event (ErrorKind::codec, exact codec path) and
//     no callback.
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
#include <vector>

#include "ams_gra/lacal/runtime.hpp"
#include "amti_settings_documents.hpp"
#include "check.hpp"
#include "oam.hpp"
#include "raw_owp_client.hpp"
#include "service_api.hpp"
#include "service_codec.hpp"

// The typed payload is bound to the GENERATED codec, never a handwritten one.
AMS_GRA_LACAL_PAYLOAD_CODEC(::programs::oam::AMTISettingsCommandMT, ::service_codec::encode_AMTISettingsCommandMT,
                            ::service_codec::decode_AMTISettingsCommandMT)

namespace m = ::programs::oam;
namespace sc = ::service_codec;
namespace lacal = ams_gra::lacal;
namespace svc = ::service_api::function_loop;
using std::chrono::milliseconds;

namespace {

const std::string kService = "amti-settings-loop";
const std::string kTopic = "amti.settings";
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

m::AMTISettingsCommandMT decode_ok(const std::string& json) {
    auto result = sc::decode_AMTISettingsCommandMT(json);
    if (!result.ok()) {
        std::cerr << "test setup: decode failed at " << result.error().path << ": " << result.error().message << '\n';
        std::exit(2);
    }
    return std::move(result).value();
}

std::string encode(const m::AMTISettingsCommandMT& value) {
    auto result = sc::encode_AMTISettingsCommandMT(value);
    return result.ok() ? result.value() : "<encode failed: " + result.error().message + ">";
}

void no_failures(const std::vector<std::string>& failures, const std::string& what) {
    for (const std::string& failure : failures) {
        std::cerr << what << ": " << failure << '\n';
    }
    CHECK(failures.empty());
}

std::string envelope(const std::string& body) { return "{\"AMTI_SettingsCommand\":" + body + "}"; }

/// Typed deliveries from the generated subscription.
struct Inbox {
    std::mutex mutex;
    std::condition_variable cv;
    std::deque<m::AMTISettingsCommandMT> values;
    void add(const m::AMTISettingsCommandMT& value) {
        {
            std::lock_guard<std::mutex> lock(mutex);
            values.push_back(value);
        }
        cv.notify_all();
    }
    std::optional<m::AMTISettingsCommandMT> take(milliseconds wait) {
        std::unique_lock<std::mutex> lock(mutex);
        if (!cv.wait_for(lock, wait, [&] { return !values.empty(); })) {
            return std::nullopt;
        }
        m::AMTISettingsCommandMT value = std::move(values.front());
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
    config.schema_version = amti::kVersion;
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

void say(const std::string& text) { std::cout << "CPP-OMS-005 " << amti::kVersion << " " << text << std::endl; }

}  // namespace

TEST_CASE(documents_and_generated_identity) {
    // The global wire element is AMTI_SettingsCommand, never AMTI_SettingsCommandMT.
    CHECK_EQ(std::string(svc::exchange_input_settings::message_name), std::string("AMTI_SettingsCommand"));
    CHECK_EQ(std::string(svc::exchange_output_settings::message_name), std::string("AMTI_SettingsCommand"));
    CHECK_EQ(std::string(svc::exchange_input_settings::message_namespace), std::string(lacal::owp::oam_namespace));
    CHECK_EQ(std::string(svc::exchange_input_settings::topic), kTopic);
    CHECK_EQ(std::string(svc::exchange_output_settings::topic), kTopic);
    CHECK_EQ(lacal::owp::global_element_name(svc::exchange_output_settings::message_namespace,
                                             svc::exchange_output_settings::message_name),
             std::string("AMTI_SettingsCommand"));
    no_failures(amti::verify_minimal(decode_ok(amti::kPresent), true), "present");
    no_failures(amti::verify_minimal(decode_ok(amti::kAbsent), false), "absent");
    no_failures(amti::verify_full(decode_ok(amti::kFull)), "full");
    CHECK_EQ(encode(decode_ok(amti::kPresent)), amti::kPresent);
    CHECK_EQ(encode(decode_ok(amti::kAbsent)), amti::kAbsent);
    CHECK_EQ(encode(decode_ok(amti::kFull)), amti::kFull);
    say("DOCUMENTS: PASSED");
}

TEST_CASE(pinned_sleet_amti_settings_roundtrip_rejections_and_recovery) {
    const std::string url = env("CPP_OMS_005_SLEET_URL");
    const Url where = split(url);
    lacal::Runtime runtime = connect_runtime(url);
    CHECK(runtime.server_info().find("\"version\":\"1.0\"") != std::string::npos);

    Inbox inbox;
    auto subscription = svc::exchange_input_settings::subscribe(
        runtime, [&](const svc::exchange_input_settings::Payload& value) { inbox.add(value); });
    CHECK(subscription.ok());
    if (!subscription.ok()) {
        std::cerr << "subscribe: " << subscription.error().message << '\n';
        return;
    }

    raw::Client observer(where.host, where.port, kService, amti::kVersion, kWait);
    raw::Client publisher(where.host, where.port, kService, amti::kVersion, kWait);
    CHECK_EQ(observer.command("SUB wire AMTI_SettingsCommand " + kTopic), std::string("+OK"));
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
        int shape;  // 0 = UnassignAll present "", 1 = absent, 2 = full
    };
    const std::vector<Typed> typed = {
        {"UnassignAll present \"\"", amti::kPresent, 0},
        {"UnassignAll absent", amti::kAbsent, 1},
        {"full MessageData", amti::kFull, 2},
    };
    for (const Typed& item : typed) {
        const m::AMTISettingsCommandMT value = decode_ok(item.canonical);
        lacal::Status published = svc::exchange_output_settings::publish(runtime, value);
        CHECK(published.ok());                          // Sleet validated and acknowledged (+OK)
        const std::string wire = observe();
        CHECK_EQ(wire, envelope(item.canonical));       // actual MSG bytes, byte for byte
        CHECK_EQ(wire.find("\"UnassignAll\""), item.shape == 1 ? std::string::npos : wire.find("\"UnassignAll\":\"\""));
        auto received = inbox.take(kWait);
        CHECK(received.has_value());
        if (received) {
            no_failures(item.shape == 2 ? amti::verify_full(*received) : amti::verify_minimal(*received, item.shape == 0),
                        item.name);
            CHECK_EQ(encode(*received), item.canonical);
        }
        CHECK(no_event());
        say(std::string("typed MSG ") + item.name + ": PASSED");
    }

    const m::AMTISettingsCommandMT present = decode_ok(amti::kPresent);
    const auto recover = [&](const std::string& after) {
        CHECK(svc::exchange_output_settings::publish(runtime, present).ok());
        CHECK_EQ(observe(), envelope(amti::kPresent));
        auto recovered = inbox.take(kWait);
        CHECK(recovered.has_value());
        if (recovered) {
            no_failures(amti::verify_minimal(*recovered, true), "recovery");
        }
        CHECK(no_event());
        say(after + ": same-subscription recovery PASSED");
    };

    // ---- 2. independently authored documents from the raw publisher ----------
    // Whitespace and member order differ from the canonical encoding; Sleet
    // validates and forwards the text verbatim; the generated decoder gives the
    // same typed value, which re-encodes canonically.
    {
        std::string loose = amti::with(amti::kPresent, R"(,"UnassignAll":"")", "");
        loose = amti::with(loose, R"("MessageData":{)", R"("MessageData" : { "UnassignAll" : "" , )");
        const std::string document = envelope(loose);
        CHECK_EQ(publisher.command("PUB " + kTopic + " " + document), std::string("+OK"));
        CHECK_EQ(observe(), document);
        auto received = inbox.take(kWait);
        CHECK(received.has_value());
        if (received) {
            no_failures(amti::verify_minimal(*received, true), "raw reordered");
            CHECK_EQ(encode(*received), amti::kPresent);
        }
        CHECK(no_event());
        say("raw reordered UnassignAll \"\" accepted by Sleet and decoded: PASSED");
    }
    {
        const std::string document = envelope(amti::kAbsent);
        CHECK_EQ(publisher.command("PUB " + kTopic + " " + document), std::string("+OK"));
        CHECK_EQ(observe(), document);
        auto received = inbox.take(kWait);
        CHECK(received.has_value());
        if (received) {
            no_failures(amti::verify_minimal(*received, false), "raw absent");
            CHECK_EQ(encode(*received), amti::kAbsent);
        }
        CHECK(no_event());
        say("raw UnassignAll absent accepted by Sleet and decoded as absent: PASSED");
    }

    // ---- 3. malformed documents ---------------------------------------------
    struct Bad {
        std::string name;
        std::string document;      // the payload body
        bool sleet_delivers;       // the pinned Sleet accepts and forwards it
        std::string diagnostic;    // codec path (delivered) or -ERR fragment (rejected)
        std::string codec_message; // delivered only: fragment of the decoder's message
    };
    const auto unassign = [](const std::string& raw) {
        return amti::with(amti::kPresent, R"("UnassignAll":"")", "\"UnassignAll\":" + raw);
    };
    std::string too_many = R"("MessageOutput":[)";
    for (std::size_t i = 0; i <= amti::kMessageOutputMax; ++i) {
        too_many += i == 0 ? R"("ENTITY")" : R"(,"ENTITY")";
    }
    too_many += "]";
    // The -ERR fragments are the pinned Sleet's own validation diagnostics
    // (observed, not chosen): pattern before length, so " " is a pattern
    // violation of [a-zA-Z]{0}.
    const std::string data = "AMTI_SettingsCommand.MessageData";
    const std::string unassign_path = "path: \"" + data + ".UnassignAll\"";
    const std::vector<Bad> bad = {
        // Sleet rejects (schema validation of the EmptyType facets / type).
        {"UnassignAll \" \"", unassign(R"(" ")"), false, "FacetViolation { " + unassign_path + ", facet: \"pattern\"", ""},
        {"UnassignAll \"x\"", unassign(R"("x")"), false, "FacetViolation { " + unassign_path + ", facet: \"pattern\"", ""},
        {"UnassignAll number", unassign("0"), false,
         "TypeMismatch { " + unassign_path + ", expected: \"string\", found: \"number\" }", ""},
        {"UnassignAll null", unassign("null"), false,
         "TypeMismatch { " + unassign_path + ", expected: \"string\", found: \"null\" }", ""},
        {"UnassignAll array", unassign("[]"), false, "OccurrenceMismatch { " + unassign_path + ", expected_array: false }",
         ""},
        {"UnassignAll wrong case key", amti::with(amti::kPresent, R"("UnassignAll")", R"("Unassignall")"), false,
         "UnknownMember { path: \"" + data + "\", member: \"Unassignall\" }", ""},
        {"duplicate UnassignAll (last is \" \")",
         amti::with(amti::kPresent, R"("UnassignAll":"")", R"("UnassignAll":"","UnassignAll":" ")"), false,
         "FacetViolation { " + unassign_path + ", facet: \"pattern\"", ""},
        {"CommandState wrong case", amti::with(amti::kPresent, R"("CommandState":"NEW")", R"("CommandState":"new")"),
         false, "FacetViolation { path: \"" + data + ".CommandState\", facet: \"enumeration\"", ""},
        {"missing CommandState", amti::with(amti::kPresent, R"("CommandState":"NEW",)", ""), false,
         "MissingRequiredMember { path: \"" + data + "\", member: \"CommandState\" }", ""},
        {"CapabilityRank above 65535", amti::with(amti::kFull, R"("CapabilityRank":65535)", R"("CapabilityRank":65536)"),
         false, "path: \"" + data + ".CapabilityRank[0].CapabilityRank\", facet: \"builtinRange\"", ""},
        {"MessageOutput above maxOccurs", amti::with(amti::kFull, amti::kOutputs, too_many), false,
         "OccurrenceMismatch { path: \"" + data + ".MessageOutput\", expected_array: true }", ""},
        // Sleet delivers (its JSON reader keeps the LAST duplicate, which is a
        // valid ""), the generated decoder rejects duplicate members.
        {"duplicate UnassignAll (last is \"\")",
         amti::with(amti::kPresent, R"("UnassignAll":"")", R"("UnassignAll":" ","UnassignAll":"")"), true, "$",
         "duplicate object member"},
    };
    std::uint64_t decode_errors = 0;
    for (const Bad& item : bad) {
        const std::string document = envelope(item.document);
        const std::string status = publisher.command("PUB " + kTopic + " " + document);
        if (item.sleet_delivers) {
            CHECK_EQ(status, std::string("+OK"));
            CHECK_EQ(observe(), document);  // the MSG really crossed Sleet
            auto event = runtime.poll_event(kWait);
            CHECK(event.has_value() && event->kind == lacal::EventKind::decode_error);
            if (event) {
                ++decode_errors;
                CHECK(event->error.kind == lacal::ErrorKind::codec);  // subscription decode error, not broker
                CHECK_EQ(event->message_name, std::string("AMTI_SettingsCommand"));
                CHECK_EQ(event->sid, subscription->id());
                CHECK_EQ(event->error.codec_path, item.diagnostic);
                CHECK(event->error.message.find(item.codec_message) != std::string::npos);
                say(item.name + ": Sleet +OK and MSG -> generated decoder rejected at " + event->error.codec_path +
                    ": " + event->error.message);
            }
        } else {
            CHECK(status.rfind("-ERR Invalid-Message", 0) == 0);
            CHECK(status.find(item.diagnostic) != std::string::npos);
            CHECK(!observer.receive(kQuiet).has_value());
            CHECK(!runtime.poll_event(kQuiet).has_value());
            say(item.name + ": Sleet rejected: " + status.substr(0, 160));
        }
        CHECK(!inbox.take(kQuiet).has_value());  // never a typed delivery
        recover(item.name);
    }
    {
        const std::string status =
            publisher.command("PUB " + kTopic + " {\"AMTI_SettingsCommandMT\":" + amti::kPresent + "}");
        CHECK(status.rfind("-ERR Invalid-Message", 0) == 0 && status.find("AMTI_SettingsCommandMT") != std::string::npos);
        CHECK(!observer.receive(kQuiet).has_value());
        say("wrong global element AMTI_SettingsCommandMT: Sleet rejected: " + status.substr(0, 120));
        recover("wrong global element");
    }

    // ---- 4. runtime-level broker rejection vs local codec rejection ---------
    {
        lacal::Status denied = runtime.publish(svc::exchange_output_settings::message_namespace,
                                               svc::exchange_output_settings::message_name, "unauthorized-topic", present);
        CHECK(denied.error().kind == lacal::ErrorKind::broker_rejected);
        CHECK_EQ(denied.error().owp_code, std::string("Illegal-State"));
        say("runtime PUB unauthorized topic: broker_rejected -ERR " + denied.error().owp_code + " " +
            denied.error().owp_details);
        lacal::Status invalid_topic = runtime.publish(svc::exchange_output_settings::message_namespace,
                                                      svc::exchange_output_settings::message_name, "invalid topic", present);
        CHECK(invalid_topic.error().kind == lacal::ErrorKind::invalid_argument);
        // Local codec error: a CommandState outside its enumeration (a
        // well-defined scoped-enum value) is refused by the generated encoder;
        // nothing is sent. (EmptyType itself cannot hold a non-empty value:
        // it is constructible only through its checked create.)
        m::AMTISettingsCommandMT forged = present;
        forged.messagedata.commandstate = static_cast<m::CommandStateEnum>(99);
        lacal::Status codec = svc::exchange_output_settings::publish(runtime, forged);
        CHECK(codec.error().kind == lacal::ErrorKind::codec);
        CHECK_EQ(codec.error().codec_path, std::string("AMTI_SettingsCommandMT.MessageData.CommandState"));
        say("local codec rejection at " + codec.error().codec_path + ": " + codec.error().message + " (nothing sent)");
        CHECK(!observer.receive(kQuiet).has_value());
        CHECK(runtime.connected());
        recover("runtime-level rejections");
    }

    // ---- 5. UNSUB and cleanup -------------------------------------------------
    CHECK(subscription->unsubscribe().ok());  // verbose: Sleet acknowledged UNSUB
    CHECK(svc::exchange_output_settings::publish(runtime, present).ok());
    CHECK_EQ(observe(), envelope(amti::kPresent));
    CHECK(!inbox.take(kQuiet).has_value());  // no callback after UNSUB
    CHECK(no_event());
    CHECK(runtime.close().ok());
    CHECK(check::threads_named("ams-lacal") == 0);
    CHECK(check::eventually([&] { return connections_to(where.port) == 2; }));  // only the raw clients remain
    CHECK_EQ(observer.command("UNSUB wire"), std::string("+OK"));
    const lacal::Stats stats = runtime.stats();
    // 3 typed + 2 raw + one recovery per malformed case (+ wrong element) + runtime-level.
    CHECK_EQ(stats.delivered, 3U + 2U + bad.size() + 1U + 1U);
    CHECK_EQ(stats.decode_errors, decode_errors);
    CHECK_EQ(decode_errors, 1U);
    CHECK_EQ(stats.unknown_subscription, 0U);
    CHECK_EQ(stats.handler_exceptions, 0U);
    say("SLEET AND RECOVERY: PASSED");
}

int main(int argc, char** argv) {
    return check::run_all(("CPP_OMS_005_REAL_AMTI_SETTINGS_COMPLETE uci=" + std::to_string(UCI_AMTI_VERSION)).c_str(),
                          argc, argv);
}
