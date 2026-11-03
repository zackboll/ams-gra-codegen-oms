// CPP-OMS-005 consumer: compiled against the GENERATED model and service_codec
// headers for the authentic pinned UCI `AMTI_SettingsCommand` (2.5 or 2.6).
//
//   -DUCI_AMTI_VERSION=25 | 26  -I <generated> -I tests/cpp-codec
//   amti_settings_consumer   -> CPP_OMS_005_AMTI_SETTINGS uci=NN checks=N rejections=M
//
// Built by scripts/check-cpp-oms-005-amti-settings.sh with strict C++17 flags
// and sanitizers. Every document and expected encoding is hand-written (see
// amti_settings_documents.hpp); none is produced by the encoder under test.
#include <cstddef>
#include <cstdlib>
#include <iostream>
#include <optional>
#include <string>
#include <string_view>
#include <type_traits>
#include <vector>

#include "amti_settings_documents.hpp"
#include "oam.hpp"
#include "service_api.hpp"
#include "service_codec.hpp"

namespace m = ::programs::oam;
namespace sc = ::service_codec;

namespace {

int failures = 0;
int checks = 0;
int rejections = 0;

#define CHECK(condition)                                                   \
    do {                                                                   \
        ++checks;                                                          \
        if (!(condition)) {                                                \
            ++failures;                                                    \
            std::cerr << __FILE__ << ':' << __LINE__                       \
                      << ": CHECK failed: " << #condition << '\n';         \
        }                                                                  \
    } while (false)

using Message = m::AMTISettingsCommandMT;

// The authentic EmptyType carrier: only `create`, and the optional member is
// an optional carrier (present "" and absent are distinct states).
static_assert(!std::is_default_constructible<m::EmptyType>::value, "EmptyType has no default constructor");
static_assert(!std::is_constructible<m::EmptyType, std::string>::value, "EmptyType only via create");
static_assert(std::is_same<decltype(m::AMTISettingsCommandMDT::unassignall), std::optional<m::EmptyType>>::value,
              "UnassignAll is std::optional<EmptyType>");

void report(const std::vector<std::string>& found, const char* what) {
    ++checks;
    if (!found.empty()) {
        ++failures;
        for (const std::string& failure : found) {
            std::cerr << what << ": typed check failed: " << failure << '\n';
        }
    }
}

std::optional<Message> decode(const std::string& json) {
    auto result = sc::decode_AMTISettingsCommandMT(json);
    ++checks;
    if (!result.ok()) {
        ++failures;
        std::cerr << "rejected valid document at " << result.error().path << ": " << result.error().message << "\n  "
                  << json << '\n';
        return std::nullopt;
    }
    return std::move(result).value();
}

void encodes_to(const Message& value, const std::string& expected, const char* what) {
    const auto encoded = sc::encode_AMTISettingsCommandMT(value);
    ++checks;
    if (!encoded.ok() || encoded.value() != expected) {
        ++failures;
        std::cerr << what << ": encoding mismatch\n  expected " << expected << "\n  actual   "
                  << (encoded.ok() ? encoded.value() : encoded.error().path + ": " + encoded.error().message) << '\n';
    }
}

void reject(const std::string& json, const std::string& path, const std::string& needle) {
    const auto decoded = sc::decode_AMTISettingsCommandMT(json);
    ++checks;
    ++rejections;
    if (decoded.ok()) {
        ++failures;
        std::cerr << "accepted invalid document (expected " << path << ")\n  " << json << '\n';
        return;
    }
    if (decoded.error().path != path || decoded.error().message.find(needle) == std::string::npos) {
        ++failures;
        std::cerr << "wrong rejection\n  expected " << path << ": ..." << needle << "...\n  actual   "
                  << decoded.error().path << ": " << decoded.error().message << "\n  " << json << '\n';
    }
}

const std::string kData = amti::kRoot + ".MessageData";

/// The minimal document with `"UnassignAll":<raw>`.
std::string unassign(const std::string& raw) {
    return amti::with(amti::kPresent, R"("UnassignAll":"")", "\"UnassignAll\":" + raw);
}

void unassign_all() {
    // 1-3. Present "" decodes to the checked EmptyType and re-encodes exactly.
    if (auto value = decode(amti::kPresent)) {
        report(amti::verify_minimal(*value, true), "present");
        CHECK(value->messagedata.unassignall.has_value());
        const std::optional<m::EmptyType> empty = m::EmptyType::create("");
        CHECK(empty.has_value());
        CHECK(empty && value->messagedata.unassignall && value->messagedata.unassignall->value() == empty->value());
        encodes_to(*value, amti::kPresent, "present");
    }
    // 4. Absent stays absent (no "" is invented) in both directions.
    if (auto value = decode(amti::kAbsent)) {
        report(amti::verify_minimal(*value, false), "absent");
        encodes_to(*value, amti::kAbsent, "absent");
        // The same value built through the model API.
        Message built = *value;
        built.messagedata.unassignall = m::EmptyType::create("");
        CHECK(built.messagedata.unassignall.has_value());
        encodes_to(built, amti::kPresent, "model-built present");
        built.messagedata.unassignall.reset();
        encodes_to(built, amti::kAbsent, "model-built absent");
    }
    // 5. Everything but the zero-length string is rejected at the real path.
    const std::string checked = "value rejected by the generated model's EmptyType check";
    for (const char* raw : {R"(" ")", R"("x")", R"("A")", R"("  ")", R"("\t")", R"("\n")", R"("\u0000")", R"("é")"}) {
        reject(unassign(raw), amti::kUnassignPath, checked);
    }
    reject(unassign(R"("\u0020")"), amti::kUnassignPath, checked);
    reject(unassign("0"), amti::kUnassignPath, "expected string, found number");
    reject(unassign("null"), amti::kUnassignPath, "expected string, found null");
    reject(unassign("[]"), amti::kUnassignPath, "expected string, found array");
    reject(unassign(R"([""])"), amti::kUnassignPath, "expected string, found array");
    reject(unassign("{}"), amti::kUnassignPath, "expected string, found object");
    reject(unassign("true"), amti::kUnassignPath, "expected string, found boolean");
    // Duplicate UnassignAll, whichever value comes last, is a JSON error here.
    reject(amti::with(amti::kPresent, R"("UnassignAll":"")", R"("UnassignAll":" ","UnassignAll":"")"), "$",
           "duplicate object member");
    reject(amti::with(amti::kPresent, R"("UnassignAll":"")", R"("UnassignAll":"","UnassignAll":"")"), "$",
           "duplicate object member");
    // Exact, case-sensitive wire name; no Clark form in the OAM namespace.
    for (const char* key : {"Unassignall", "unassignAll", "UNASSIGNALL", "Unassign_All",
                            "{https://www.vdl.afrl.af.mil/programs/oam}UnassignAll"}) {
        reject(amti::with(amti::kPresent, R"("UnassignAll":"")", "\"" + std::string(key) + "\":\"\""), kData,
               "unknown member \"" + std::string(key) + "\"");
    }
}

void full_document() {
    if (auto value = decode(amti::kFull)) {
        report(amti::verify_full(*value), "full");
        encodes_to(*value, amti::kFull, "full");
    }
    // Whitespace and a different member order decode to the same value and
    // re-encode canonically (schema order of THIS release).
    std::string loose = amti::with(amti::kFull, R"(,"UnassignAll":"")", "");
    loose = amti::with(loose, R"({"CommandID":)", R"({ "UnassignAll" : "" ,
        "CommandID" :)");
    if (auto value = decode(loose)) {
        report(amti::verify_full(*value), "reordered");
        encodes_to(*value, amti::kFull, "reordered");
    }
    // Release-specific MessageOutput domain and cardinality.
#if UCI_AMTI_VERSION == 25
    reject(amti::with(amti::kFull, R"("ENTITY"])", R"("ACTION_ACTIVITY"])"), kData + ".MessageOutput[1]",
           "not a member of enumeration AMTI_MessageOutputsEnum");
#else
    if (auto value = decode(amti::with(amti::kFull, R"("ENTITY"])", R"("ACTION_ACTIVITY"])"))) {
        CHECK(value->messagedata.messageoutput.values().size() == 2);
        CHECK(value->messagedata.messageoutput.values()[1] == amti::MessageOutput::ACTIONACTIVITY);
    }
#endif
    std::string many = R"("MessageOutput":[)";
    for (std::size_t i = 0; i <= amti::kMessageOutputMax; ++i) {
        many += i == 0 ? R"("ENTITY")" : R"(,"ENTITY")";
    }
    reject(amti::with(amti::kFull, amti::kOutputs, many + "]"), kData + ".MessageOutput",
           std::to_string(amti::kMessageOutputMax + 1) + " occurrence(s) rejected by the generated sequence constructor");
    reject(amti::with(amti::kFull, amti::kOutputs, R"("MessageOutput":"ENTITY")"), kData + ".MessageOutput",
           "expected array, found string");
    reject(amti::with(amti::kFull, R"("ENTITY"])", R"("Entity"])"), kData + ".MessageOutput[1]",
           "not a member of enumeration");
}

void representative_fields() {
    const std::string& full = amti::kFull;
    // Missing required members.
    reject(amti::with(full, R"("CommandState":"UPDATE",)", ""), kData, "missing required member \"CommandState\"");
    reject(amti::with(amti::kAbsent, R"(,"SubsystemID":{"UUID":")" + amti::kSubsystem + R"("})", ""), kData,
           "missing required member \"SubsystemID\"");
    reject(amti::with(full, amti::kHeader + ",", ""), amti::kRoot, "missing required member \"MessageHeader\"");
    reject(amti::with(full, R"(,"SystemName":"Sys 1")", ""), kData + ".RepresentationKey",
           "missing required member \"SystemName\"");
    reject(amti::with(full, R"([{"GovernmentIdentifier":"USA"}])", "[]"), amti::kRoot + ".SecurityInformation.OwnerProducer",
           "0 occurrence(s) rejected by the generated sequence constructor");
    // Malformed lexical values.
    reject(amti::with(full, R"("CommandState":"UPDATE")", R"("CommandState":"update")"), kData + ".CommandState",
           "not a member of enumeration CommandStateEnum");
    reject(amti::with(full, R"("CommandID":{"UUID":")" + amti::kCommand, R"("CommandID":{"UUID":"11111111-2222)"),
           kData + ".CommandID.UUID", "value rejected by the generated model's");
    reject(amti::with(full, amti::kStamp, "2026-13-01T00:00:00Z"), amti::kRoot + ".MessageHeader.Timestamp",
           "value rejected by the generated model's");
    reject(amti::with(full, "\"" + amti::kVersion + "\"", R"("2.5")"), amti::kRoot + ".MessageHeader.SchemaVersion",
           "value rejected by the generated model's");
    reject(amti::with(full, R"("CapabilityRank":65535)", R"("CapabilityRank":65536)"),
           kData + ".CapabilityRank[0].CapabilityRank", "value rejected by the generated model's domain check");
    reject(amti::with(full, R"("CapabilityRank":65535)", R"("CapabilityRank":-1)"),
           kData + ".CapabilityRank[0].CapabilityRank", "integer is out of range");
    reject(amti::with(full, R"("CapabilityRank":65535)", R"("CapabilityRank":"65535")"),
           kData + ".CapabilityRank[0].CapabilityRank", "expected number, found string");
    reject(amti::with(full, R"("CapabilityRank":65535)", R"("CapabilityRank":65535.0)"),
           kData + ".CapabilityRank[0].CapabilityRank", "");
    reject(amti::with(full, R"("EMCON_LEVEL_3")", R"("EMCON_LEVEL_6")"), kData + ".DefaultEmcon.EmconLevel",
           "not a member of enumeration EmconLevelEnum");
    reject(amti::with(full, R"({"EmconLevel":"EMCON_LEVEL_3"})", "{}"), kData + ".DefaultEmcon",
           "a Choice requires exactly one selected member");
    reject(amti::with(full, R"("ElectronicProtectionOptionStatus":"DISABLE")", R"("ElectronicProtectionOptionStatus":"OFF")"),
           kData + ".ElectronicProtectionOptions[0].ElectronicProtectionOptions[0].ElectronicProtectionOptionStatus",
           "not a member of enumeration");
    // Exact wire names elsewhere (underscored XSD names, not C++ spellings).
    reject(amti::with(full, R"("MDF_ID")", R"("MDFID")"), kData + ".ActivateMDF[0]", "unknown member \"MDFID\"");
    reject(amti::with(full, R"("ActivateMDF")", R"("activatemdf")"), kData, "unknown member \"activatemdf\"");
    reject("{\"AMTI_SettingsCommand\":" + amti::kPresent + "}", amti::kRoot, "unknown member \"AMTI_SettingsCommand\"");
    reject(amti::with(amti::kPresent, R"("MessageData":{)", R"("MessageData":{"$type":"AMTI_SettingsCommandMT",)"), kData,
           "\"$type\" does not name the concrete type being decoded");
    if (auto typed = decode(amti::with(amti::kPresent, R"("MessageData":{)", R"("MessageData":{"$type":"AMTI_SettingsCommandMDT",)"))) {
        encodes_to(*typed, amti::kPresent, "own $type");  // never emitted
    }
    // Local encode-side rejection: an enumerator outside the enumeration (a
    // well-defined scoped-enum value) is refused, with the schema path.
    if (auto value = decode(amti::kPresent)) {
        Message forged = *value;
        forged.messagedata.commandstate = static_cast<m::CommandStateEnum>(99);
        const auto encoded = sc::encode_AMTISettingsCommandMT(forged);
        ++checks;
        ++rejections;
        if (encoded.ok() || encoded.error().path != kData + ".CommandState" ||
            encoded.error().message.find("not an enumerator") == std::string::npos) {
            ++failures;
            std::cerr << "forged CommandState was not refused by the encoder\n";
        }
    }
}

}  // namespace

int main() {
    unassign_all();
    full_document();
    representative_fields();
    if (failures != 0) {
        std::cerr << failures << " failure(s) of " << checks << " checks\n";
        return 1;
    }
    std::cout << "CPP_OMS_005_AMTI_SETTINGS uci=" << UCI_AMTI_VERSION << " checks=" << checks
              << " rejections=" << rejections << '\n';
    return 0;
}
