// CPP-OMS-002 consumer: compiled against the GENERATED model and service_codec
// headers for the authentic pinned UCI `Log` message (UCI 2.5 or 2.6).
//
//   -DUCI_LOG_VERSION=25   or   -DUCI_LOG_VERSION=26
//   log_consumer <task093-real-logNN.json>
//
// Built by scripts/check-cpp-oms-002-real-log.sh with strict C++17 flags. Exit
// status 0 and the final CPP_OMS_002_LOG_COMPLETE line mean every check passed.
//
// Independence: every expected JSON document below is authored by hand in this
// file (compact, schema order). None is produced by the encoder under test; the
// decoder is exercised on the repository's existing Task 093 golden file AND on
// these literals.
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <iterator>
#include <optional>
#include <sstream>
#include <string>
#include <string_view>
#include <type_traits>
#include <variant>
#include <vector>

#include "oam.hpp"
#include "service_api.hpp"
#include "service_codec.hpp"

#ifndef UCI_LOG_VERSION
#error "define UCI_LOG_VERSION=25 or 26"
#endif

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

#if UCI_LOG_VERSION == 25
const std::string kVersion = "002.5.0";
// 2.5: DeclassDate is a direct xs:dateTime, so a numeric offset is a valid value.
const std::string kDeclass = "2030-01-01T00:00:00-05:00";
constexpr bool kDeclassAcceptsOffset = true;
// 2.5 WhitespaceVisibleString*: whiteSpace=collapse, minLength 0.
const std::string kWsInput = "  a \t b\n";
const std::string kWsStored = "a b";
const std::string kWsWire = R"("a b")";
const std::string kWsJsonIn = R"("  x \t\n y ")";
const std::string kWsJsonStored = "x y";
constexpr bool kWsCollapses = true;
#elif UCI_LOG_VERSION == 26
const std::string kVersion = "002.6.0";
// 2.6: DeclassDate is the named Zulu DateTimeType, so only `...Z` is valid.
const std::string kDeclass = "2030-01-01T00:00:00Z";
constexpr bool kDeclassAcceptsOffset = false;
// 2.6 WhitespaceVisibleString*: whiteSpace=preserve, minLength 1, LF allowed, TAB not.
const std::string kWsInput = "a\n b";
const std::string kWsStored = "a\n b";
const std::string kWsWire = R"("a\n b")";
const std::string kWsJsonIn = R"("  x \n y ")";
const std::string kWsJsonStored = "  x \n y ";
constexpr bool kWsCollapses = false;
#else
#error "UCI_LOG_VERSION must be 25 or 26"
#endif

// The carrier types differ between releases: 2.5 has a direct xs:duration and
// a direct xs:hexBinary member; 2.6 has the named DurationType and HexBinaryType.
using DurationCarrier = typename decltype(m::LogMDT::serviceuptime)::value_type;
using DataCarrier = typename decltype(m::LogMDT::data)::value_type;
// DeclassDate is a direct xs:dateTime (XmlSchemaDateTime) in 2.5; 2.6 may name it.
using DeclassCarrier = typename decltype(m::SecurityInformationType::declassdate)::value_type;

template <typename D>
D make_data(std::vector<std::uint8_t> octets) {
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

/// A generated factory that must accept its argument (test setup only).
template <typename T, typename V>
T must(V value, int line = __builtin_LINE()) {
    auto created = T::create(std::move(value));
    if (!created) {
        std::cerr << "test setup (line " << line << "): a generated factory rejected its argument\n";
        std::exit(2);
    }
    return *created;
}

// ---- independently authored expected documents ------------------------------

const std::string kSecurity =
    R"("SecurityInformation":{"Classification":"U","OwnerProducer":[{"GovernmentIdentifier":"USA"},{"NATO_SpecialWord":"NATO:EXAMPLE"}]})";
const std::string kHeaderHead =
    R"("MessageHeader":{"SystemID":{"UUID":"12345678-1234-1234-8234-123456789abc"},"Timestamp":"2026-01-02T03:04:05Z","SchemaVersion":")";
const std::string kHeaderTail = R"(","Mode":"LIVE"})";
const std::string kDataHead =
    R"("MessageData":{"Timestamp":"2026-01-02T03:04:06Z",)";
const std::string kDataTail =
    R"("Severity":"INFO","Label":"Task093","Details":"Independent real Log Binary codec qualification")";

std::string document(const std::string& security, const std::string& uptime,
                     const std::string& data_member, const std::string& extra_data = "") {
    return "{" + security + "," + kHeaderHead + kVersion + kHeaderTail + "," + kDataHead + uptime +
           extra_data + kDataTail + data_member + "}}";
}

const std::string kUptime = R"("ServiceUpTime":"PT01.2300S",)";
const std::string kData = R"(,"Data":"0080FF")";
const std::string kGolden = document(kSecurity, kUptime, kData);

/// Replace exactly one occurrence (a test-authoring error otherwise).
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

std::string read_file(const char* path) {
    std::ifstream stream(path, std::ios::binary);
    if (!stream) {
        std::cerr << "cannot read " << path << '\n';
        std::exit(2);
    }
    return std::string(std::istreambuf_iterator<char>(stream), std::istreambuf_iterator<char>());
}

/// A rejected document must fail with `path_part` in the path and `message_part`
/// in the message.
void reject(const std::string& name, const std::string& json, std::string_view path_part,
            std::string_view message_part) {
    ++rejections;
    const auto result = sc::decode_LogMT(json);
    if (result.ok()) {
        ++checks;
        ++failures;
        std::cerr << "REJECTION MISSING: " << name << '\n';
        return;
    }
    const sc::CodecError& error = result.error();
    const bool path_ok = error.path.find(path_part) != std::string::npos;
    const bool message_ok = error.message.find(message_part) != std::string::npos;
    ++checks;
    if (!path_ok || !message_ok) {
        ++failures;
        std::cerr << "REJECTION WRONG: " << name << "\n  path:    " << error.path
                  << "\n  message: " << error.message << "\n  wanted path ~ '" << path_part
                  << "', message ~ '" << message_part << "'\n";
    }
}

m::LogMT decode_ok(const std::string& json) {
    auto result = sc::decode_LogMT(json);
    if (!result.ok()) {
        std::cerr << "test setup: decode failed at " << result.error().path << ": "
                  << result.error().message << "\n  " << json << '\n';
        std::exit(2);
    }
    return std::move(result).value();
}

std::string encode_ok(const m::LogMT& value) {
    auto result = sc::encode_LogMT(value);
    if (!result.ok()) {
        std::cerr << "test setup: encode failed at " << result.error().path << ": "
                  << result.error().message << '\n';
        std::exit(2);
    }
    return std::move(result).value();
}

/// Decode `json`, re-encode, and require the independently written text back.
void round_trip(const std::string& name, const std::string& json) {
    const auto result = sc::decode_LogMT(json);
    ++checks;
    if (!result.ok()) {
        ++failures;
        std::cerr << "ROUND TRIP DECODE FAILED: " << name << " at " << result.error().path << ": "
                  << result.error().message << '\n';
        return;
    }
    const std::string again = encode_ok(result.value());
    ++checks;
    if (again != json) {
        ++failures;
        std::cerr << "ROUND TRIP DIFFERS: " << name << "\n  expected " << json << "\n  actual   "
                  << again << '\n';
    }
}

// ---- typed inspection of the decoded golden ---------------------------------

void inspect_golden(const m::LogMT& log) {
    const auto& security = log.securityinformation;
    CHECK(security.classification == m::ClassificationEnum::U);
    CHECK(security.ownerproducer.values().size() == 2);
    const auto* government =
        std::get_if<m::OwnerProducerChoiceType::GovernmentIdentifier>(&security.ownerproducer.values()[0].value);
    CHECK(government != nullptr);
    if (government != nullptr) {
        CHECK(government->value == m::OwnerProducerEnum::USA);
    }
    const auto* nato =
        std::get_if<m::OwnerProducerChoiceType::NATOSpecialWord>(&security.ownerproducer.values()[1].value);
    CHECK(nato != nullptr);
    if (nato != nullptr) {
        CHECK(nato->value.value() == "NATO:EXAMPLE");
    }
    CHECK(!security.joint.has_value());
    CHECK(security.sci_controls.values().empty());
    CHECK(!security.declassdate.has_value());
    CHECK(!security.classifiedby.has_value());

    const auto& header = log.messageheader;
    CHECK(header.systemid.uuid.value() == "12345678-1234-1234-8234-123456789abc");
    CHECK(!header.systemid.descriptivelabel.has_value());
    CHECK(header.timestamp.value() == "2026-01-02T03:04:05Z");
    CHECK(header.schemaversion.value() == kVersion);
    CHECK(header.mode == m::MessageModeEnum::LIVE);
    CHECK(!header.serviceid.has_value());
    CHECK(!header.missionid.has_value());

    const auto& data = log.messagedata;
    CHECK(data.timestamp.value() == "2026-01-02T03:04:06Z");
    CHECK(data.serviceuptime.has_value());
    if (data.serviceuptime.has_value()) {
        // The lexical spelling is preserved, not canonicalised to PT1.23S.
        CHECK(data.serviceuptime->value() == "PT01.2300S");
    }
    CHECK(!data.serviceid.has_value());
    CHECK(!data.subsystemid.has_value());
    CHECK(data.componentid.values().empty());
    CHECK(data.capabilityid.values().empty());
    CHECK(data.severity == m::LogSeverityEnum::INFO);
    CHECK(data.label.value() == "Task093");
    CHECK(data.details.value() == "Independent real Log Binary codec qualification");
    CHECK(data.data.has_value());
    if (data.data.has_value()) {
        const std::vector<std::uint8_t> expected{0x00, 0x80, 0xFF};
        CHECK(octets_of(*data.data) == expected);
    }
}

m::UniversallyUniqueIdentifierType uuid(const char* text) {
    return must<m::UniversallyUniqueIdentifierType>(std::string_view(text));
}

m::VisibleString256Type label256(const char* text) {
    return must<m::VisibleString256Type>(std::string_view(text));
}

}  // namespace

int main(int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: log_consumer <golden.json>\n";
        return 2;
    }

    // ---- 1. decode the existing authentic golden, inspect, re-encode --------
    const std::string golden_file = read_file(argv[1]);
    const m::LogMT golden = decode_ok(golden_file);
    inspect_golden(golden);
    CHECK(encode_ok(golden) == kGolden);
    // The hand-written compact literal decodes to the same typed content.
    inspect_golden(decode_ok(kGolden));
    round_trip("golden", kGolden);

    // ---- 2. optional / empty / populated hexBinary --------------------------
    {
        m::LogMT log = golden;
        log.messagedata.data = std::nullopt;
        CHECK(encode_ok(log) == document(kSecurity, kUptime, ""));
        round_trip("data absent", document(kSecurity, kUptime, ""));
        CHECK(!decode_ok(document(kSecurity, kUptime, "")).messagedata.data.has_value());

        log.messagedata.data = make_data<DataCarrier>({});
        CHECK(encode_ok(log) == document(kSecurity, kUptime, R"(,"Data":"")"));
        round_trip("data empty", document(kSecurity, kUptime, R"(,"Data":"")"));
        const m::LogMT empty = decode_ok(document(kSecurity, kUptime, R"(,"Data":"")"));
        CHECK(empty.messagedata.data.has_value());
        CHECK(empty.messagedata.data.has_value() && octets_of(*empty.messagedata.data).empty());

        log.messagedata.data = make_data<DataCarrier>({0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x0A});
        CHECK(encode_ok(log) == document(kSecurity, kUptime, R"(,"Data":"DEADBEEF000A")"));
    }
    {
        // Lowercase and surrounding XML whitespace are accepted; output is canonical.
        const m::LogMT lower = decode_ok(with(kGolden, R"("0080FF")", R"("00ff80")"));
        CHECK(octets_of(*lower.messagedata.data) == (std::vector<std::uint8_t>{0x00, 0xFF, 0x80}));
        CHECK(encode_ok(lower) == with(kGolden, R"("0080FF")", R"("00FF80")"));
        const m::LogMT spaced = decode_ok(with(kGolden, R"("0080FF")", R"(" \t0080ff\n ")"));
        CHECK(encode_ok(spaced) == kGolden);
    }

    // ---- 3. optional Duration ------------------------------------------------
    {
        m::LogMT log = golden;
        log.messagedata.serviceuptime = std::nullopt;
        CHECK(encode_ok(log) == document(kSecurity, "", kData));
        round_trip("uptime absent", document(kSecurity, "", kData));
        log.messagedata.serviceuptime = must<DurationCarrier>(std::string_view("-P1Y2M3DT4H5M6.7S"));
        CHECK(encode_ok(log) == document(kSecurity, R"("ServiceUpTime":"-P1Y2M3DT4H5M6.7S",)", kData));
        round_trip("uptime negative", document(kSecurity, R"("ServiceUpTime":"-P1Y2M3DT4H5M6.7S",)", kData));
        // XML whitespace collapse applies on the way in; the stored text is collapsed.
        const m::LogMT padded =
            decode_ok(with(kGolden, R"("PT01.2300S")", R"("  PT1S\n")"));
        CHECK(padded.messagedata.serviceuptime->value() == "PT1S");
    }

    // ---- 4. repeated and optional records, labels ---------------------------
    {
        m::LogMT log = golden;
        std::vector<m::ComponentIDType> components;
        components.push_back(m::ComponentIDType{uuid("aaaaaaaa-bbbb-1ccc-8ddd-eeeeeeeeeeee"), label256("first")});
        components.push_back(m::ComponentIDType{uuid("00000000-0000-0000-0000-000000000000"), std::nullopt});
        log.messagedata.componentid =
            must<decltype(log.messagedata.componentid)>(std::move(components));
        log.messagedata.serviceid = m::ServiceIDType{uuid("11111111-2222-3333-8444-555555555555"),
                                                     label256("svc"), label256("1.0")};
        const std::string expected = document(
            kSecurity, kUptime, kData,
            R"("ServiceID":{"UUID":"11111111-2222-3333-8444-555555555555","DescriptiveLabel":"svc","ServiceVersion":"1.0"},)"
            R"("ComponentID":[{"UUID":"aaaaaaaa-bbbb-1ccc-8ddd-eeeeeeeeeeee","DescriptiveLabel":"first"},{"UUID":"00000000-0000-0000-0000-000000000000"}],)");
        CHECK(encode_ok(log) == expected);
        round_trip("components", expected);
        const m::LogMT again = decode_ok(expected);
        CHECK(again.messagedata.componentid.values().size() == 2);
        CHECK(again.messagedata.componentid.values()[0].uuid.value() == "aaaaaaaa-bbbb-1ccc-8ddd-eeeeeeeeeeee");
        CHECK(again.messagedata.componentid.values()[1].uuid.value() == "00000000-0000-0000-0000-000000000000");
        CHECK(!again.messagedata.componentid.values()[1].descriptivelabel.has_value());
        CHECK(again.messagedata.serviceid.has_value());
        CHECK(again.messagedata.serviceid->serviceversion->value() == "1.0");
        // A present empty repetition encodes as absence; `[]` decodes to empty.
        const m::LogMT empty = decode_ok(with(kGolden, R"("Severity")", R"("ComponentID":[],"Severity")"));
        CHECK(empty.messagedata.componentid.values().empty());
        CHECK(encode_ok(empty) == kGolden);
    }

    // ---- 5. SecurityInformation: choices, enums, bool, direct dateTime ------
    {
        m::LogMT log = golden;
        auto& security = log.securityinformation;
        security.joint = true;
        security.sci_controls =
            must<decltype(security.sci_controls)>(std::vector<m::SCIControlsEnum>{
                m::SCIControlsEnum::BURBLG, m::SCIControlsEnum::HCS});
        security.releasableto = must<decltype(security.releasableto)>(std::vector<m::ReleasableToChoiceType>{
            m::ReleasableToChoiceType{m::ReleasableToChoiceType::GovernmentIdentifier{m::ReleasableToEnum::USA}},
            m::ReleasableToChoiceType{m::ReleasableToChoiceType::NATOSpecialWord{
                must<m::NATOSpecialWordsType>(std::string_view("NATO:ABC_def-GHI"))}}});
        // Whitespace-visible: 2.5 collapses on the way in ("a b"); 2.6 preserves it.
        security.classifiedby = must<m::WhitespaceVisibleString1024Type>(std::string_view(kWsInput));
        CHECK(security.classifiedby->value() == kWsStored);
        security.declassdate = must<DeclassCarrier>(std::string_view(kDeclass));
        const std::string expected_security =
            R"("SecurityInformation":{"Classification":"U","OwnerProducer":[{"GovernmentIdentifier":"USA"},{"NATO_SpecialWord":"NATO:EXAMPLE"}],)"
            R"("Joint":true,"SCI_Controls":["BUR_BLG","HCS"],)"
            R"("ReleasableTo":[{"GovernmentIdentifier":"USA"},{"NATO_SpecialWord":"NATO:ABC_def-GHI"}],)"
            R"("ClassifiedBy":)" + kWsWire + R"(,"DeclassDate":")" + kDeclass + R"("})";
        const std::string expected = document(expected_security, kUptime, kData);
        CHECK(encode_ok(log) == expected);
        round_trip("security", expected);
        const m::LogMT again = decode_ok(expected);
        CHECK(again.securityinformation.joint.has_value() && *again.securityinformation.joint);
        CHECK(again.securityinformation.sci_controls.values().size() == 2);
        CHECK(again.securityinformation.sci_controls.values()[0] == m::SCIControlsEnum::BURBLG);
        CHECK(again.securityinformation.declassdate->value() == kDeclass);
        // Member order is irrelevant on input.
        const std::string reordered =
            R"({"MessageData":{"Details":"d","Label":"l","Severity":"WARNING","Timestamp":"2026-01-02T03:04:06Z"},)"
            R"("MessageHeader":{"Mode":"EXERCISE","SchemaVersion":")" + kVersion +
            R"(","Timestamp":"2026-01-02T03:04:05Z","SystemID":{"UUID":"12345678-1234-1234-8234-123456789abc"}},)"
            R"("SecurityInformation":{"OwnerProducer":[{"GovernmentIdentifier":"USA"}],"Classification":"S"}})";
        const m::LogMT out_of_order = decode_ok(reordered);
        CHECK(out_of_order.messagedata.severity == m::LogSeverityEnum::WARNING);
        CHECK(out_of_order.messageheader.mode == m::MessageModeEnum::EXERCISE);
        CHECK(out_of_order.securityinformation.classification == m::ClassificationEnum::S);
        CHECK(!out_of_order.messagedata.serviceuptime.has_value());
        CHECK(!out_of_order.messagedata.data.has_value());
        CHECK(encode_ok(out_of_order) ==
              R"({"SecurityInformation":{"Classification":"S","OwnerProducer":[{"GovernmentIdentifier":"USA"}]},)"
              R"("MessageHeader":{"SystemID":{"UUID":"12345678-1234-1234-8234-123456789abc"},)"
              R"("Timestamp":"2026-01-02T03:04:05Z","SchemaVersion":")" + kVersion +
              R"(","Mode":"EXERCISE"},)"
              R"("MessageData":{"Timestamp":"2026-01-02T03:04:06Z","Severity":"WARNING","Label":"l","Details":"d"}})");
    }

    // ---- 6. malformed UUIDs ---------------------------------------------------
    {
        const std::string good = R"("UUID":"12345678-1234-1234-8234-123456789abc")";
        const std::string where = "LogMT.MessageHeader.SystemID.UUID";
        const char* bad[] = {
            "12345678-1234-1234-8234-123456789ab",        // one short
            "12345678-1234-1234-8234-123456789abcd",      // one long
            "12345678123412348234123456789abc",           // no hyphens
            "12345678-1234-1234-8234-123456789abg",       // non-hex
            "12345678-1234-6234-8234-123456789abc",       // version 6
            "12345678-1234-1234-c234-123456789abc",       // variant c
            "{12345678-1234-1234-8234-123456789abc}",     // braces
            " 12345678-1234-1234-8234-123456789abc",      // leading space (preserve)
            "",                                          // empty
        };
        for (const char* text : bad) {
            reject(std::string("uuid ") + text, with(kGolden, good, std::string(R"("UUID":")") + text + "\""),
                   where, "generated model");
        }
        reject("uuid number", with(kGolden, good, R"("UUID":123)"), where, "expected string");
        reject("uuid null", with(kGolden, good, R"("UUID":null)"), where, "null");
        reject("uuid missing", with(kGolden, good, R"("DescriptiveLabel":"x")"), "LogMT.MessageHeader.SystemID",
               "missing required member \"UUID\"");
    }

    // ---- 7. invalid DateTime ---------------------------------------------------
    {
        const std::string ts = R"("Timestamp":"2026-01-02T03:04:05Z")";
        const std::string where = "LogMT.MessageHeader.Timestamp";
        const char* bad[] = {
            "2026-13-02T03:04:05Z",     // month 13
            "2026-02-30T03:04:05Z",     // 30 February
            "2026-01-02T24:04:05Z",     // hour 24 with minutes
            "2026-01-02T03:60:05Z",     // minute 60
            "2026-01-02T03:04:05",      // no zone: DateTimeType requires Z
            "2026-01-02T03:04:05+00:00",  // offset, not Z
            "2026-01-02 03:04:05Z",     // space for T
            "2026-01-02Z",              // date only
            "20260102T030405Z",         // basic format
            "",                         // empty
            "not a time",
        };
        for (const char* text : bad) {
            reject(std::string("datetime ") + text, with(kGolden, ts, std::string(R"("Timestamp":")") + text + "\""),
                   where, "generated model");
        }
        reject("datetime number", with(kGolden, ts, R"("Timestamp":1767323045)"), where, "expected string");
        // A direct (non-Zulu) xs:dateTime still has to be a dateTime.
        const std::string with_decl = with(
            kGolden, R"("Classification":"U",)", R"("Classification":"U","DeclassDate":"2030-13-01T00:00:00Z",)");
        // DeclassDate is positioned by schema order, but input order is free.
        reject("declassdate month 13", with_decl, "LogMT.SecurityInformation.DeclassDate", "generated model");
        // The release difference is real and enforced by the model: an offset is a
        // valid direct xs:dateTime (2.5) but not a Zulu DateTimeType (2.6).
        const std::string offset = with(kGolden, R"("Classification":"U",)",
                                        R"("Classification":"U","DeclassDate":"2030-01-01T00:00:00-05:00",)");
        if (kDeclassAcceptsOffset) {
            CHECK(decode_ok(offset).securityinformation.declassdate.has_value());
        } else {
            reject("declassdate offset not Zulu", offset, "LogMT.SecurityInformation.DeclassDate", "generated model");
        }
    }

    // ---- 8. invalid Duration -----------------------------------------------------
    {
        const std::string up = R"("ServiceUpTime":"PT01.2300S")";
        const std::string where = "LogMT.MessageData.ServiceUpTime";
        const char* bad[] = {"", "P", "PT", "1H", "P1H", "PT1.S", "P1Y2", "P-1D", "PT1S ago", "p1d", "P1DT"};
        for (const char* text : bad) {
            reject(std::string("duration ") + text, with(kGolden, up, std::string(R"("ServiceUpTime":")") + text + "\""),
                   where, "generated model");
        }
        reject("duration number", with(kGolden, up, R"("ServiceUpTime":3600)"), where, "expected string");
        reject("duration array", with(kGolden, up, R"("ServiceUpTime":["PT1S"])"), where, "expected string");
    }

    // ---- 9. invalid Choice values ------------------------------------------------
    {
        const std::string first = R"({"GovernmentIdentifier":"USA"})";
        const std::string second = R"({"NATO_SpecialWord":"NATO:EXAMPLE"})";
        const std::string base = "LogMT.SecurityInformation.OwnerProducer";
        reject("choice empty object", with(kGolden, first, "{}"), base + "[0]", "exactly one selected member");
        reject("choice two members", with(kGolden, first, R"({"GovernmentIdentifier":"USA","NATO_SpecialWord":"NATO:X"})"),
               base + "[0]", "exactly one selected member");
        reject("choice unknown member", with(kGolden, first, R"({"Operator":"USA"})"), base + "[0]", "unknown member");
        reject("choice wrong kind", with(kGolden, first, R"("USA")"), base + "[0]", "expected object");
        reject("choice null", with(kGolden, first, "null"), base + "[0]", "null");
        reject("choice array", with(kGolden, first, R"([{"GovernmentIdentifier":"USA"}])"), base + "[0]", "expected object");
        reject("choice enum unknown", with(kGolden, first, R"({"GovernmentIdentifier":"ZZZ"})"),
               base + "[0].GovernmentIdentifier", "not a member of enumeration");
        reject("choice enum lowercase", with(kGolden, first, R"({"GovernmentIdentifier":"usa"})"),
               base + "[0].GovernmentIdentifier", "not a member of enumeration");
        reject("choice enum number", with(kGolden, first, R"({"GovernmentIdentifier":1})"),
               base + "[0].GovernmentIdentifier", "expected string");
        reject("choice nato missing prefix", with(kGolden, second, R"({"NATO_SpecialWord":"EXAMPLE"})"),
               base + "[1].NATO_SpecialWord", "generated model");
        reject("choice nato bad char", with(kGolden, second, R"({"NATO_SpecialWord":"NATO:EX AMPLE"})"),
               base + "[1].NATO_SpecialWord", "generated model");
        reject("choice nato prefix only", with(kGolden, second, R"({"NATO_SpecialWord":"NATO:"})"),
               base + "[1].NATO_SpecialWord", "generated model");
        reject("choice nato lowercase prefix", with(kGolden, second, R"({"NATO_SpecialWord":"nato:EXAMPLE"})"),
               base + "[1].NATO_SpecialWord", "generated model");
        reject("choice bad $type", with(kGolden, first, R"({"$type":"Other","GovernmentIdentifier":"USA"})"),
               base + "[0]", "$type");
        // A matching $type is tolerated and never re-emitted.
        const m::LogMT typed = decode_ok(with(kGolden, first, R"({"$type":"OwnerProducerChoiceType","GovernmentIdentifier":"USA"})"));
        CHECK(encode_ok(typed) == kGolden);
        // Duplicate alternative member.
        reject("choice duplicate member", with(kGolden, first, R"({"GovernmentIdentifier":"USA","GovernmentIdentifier":"CAN"})"),
               "$", "duplicate object member");
    }

    // ---- 10. malformed hexBinary -----------------------------------------------
    {
        const std::string data = R"("Data":"0080FF")";
        const std::string where = "LogMT.MessageData.Data";
        reject("hex odd", with(kGolden, data, R"("Data":"0")"), where, "odd number");
        reject("hex odd long", with(kGolden, data, R"("Data":"0080F")"), where, "odd number");
        reject("hex bad digit", with(kGolden, data, R"("Data":"0G")"), where, "hexadecimal digits");
        reject("hex internal space", with(kGolden, data, R"("Data":"00 80 FF")"), where, "hexadecimal digits");
        reject("hex 0x prefix", with(kGolden, data, R"("Data":"0x0080")"), where, "hexadecimal digits");
        reject("hex non-ascii", with(kGolden, data, "\"Data\":\"00\xC3\xA9\""), where, "hexadecimal digits");
        reject("hex escape digit", with(kGolden, data, R"("Data":"00\u0000")"), where, "hexadecimal digits");
        reject("hex number", with(kGolden, data, R"("Data":128)"), where, "expected string");
        reject("hex array", with(kGolden, data, R"("Data":[0,128,255])"), where, "expected string");
        reject("hex bool", with(kGolden, data, R"("Data":true)"), where, "expected string");
        reject("hex null", with(kGolden, data, R"("Data":null)"), where, "null");
        reject("hex base64", with(kGolden, data, R"("Data":"AID/")"), where, "hexadecimal digits");
    }

    // ---- 11. duplicate members -----------------------------------------------------
    {
        reject("duplicate top-level", with(kGolden, R"("MessageHeader")", R"("MessageData":{},"MessageHeader")"), "$",
               "duplicate object member");
        reject("duplicate nested", with(kGolden, R"("Label":"Task093")", R"("Label":"Task093","Label":"other")"), "$",
               "duplicate object member");
        reject("duplicate after unescape", with(kGolden, R"("Label":"Task093")", R"("Label":"Task093","Label":"x")"),
               "$", "duplicate object member");
        reject("duplicate in unknown", with(kGolden, R"("Label":"Task093")", R"("Label":"Task093","X":{"a":1,"a":2})"), "$",
               "duplicate object member");
    }

    // ---- 12. cardinality, required members, unknown members -------------------------
    {
        const std::string owners = R"("OwnerProducer":[{"GovernmentIdentifier":"USA"},{"NATO_SpecialWord":"NATO:EXAMPLE"}])";
        const std::string sec = "LogMT.SecurityInformation";
        reject("ownerproducer empty (min 1)", with(kGolden, owners, R"("OwnerProducer":[])"), sec + ".OwnerProducer",
               "occurrence(s) rejected");
        reject("ownerproducer missing", with(kGolden, "," + owners, ""), sec + ".OwnerProducer", "");
        reject("ownerproducer not array", with(kGolden, owners, R"("OwnerProducer":{"GovernmentIdentifier":"USA"})"),
               sec + ".OwnerProducer", "expected array");
        reject("ownerproducer null", with(kGolden, owners, R"("OwnerProducer":null)"), sec + ".OwnerProducer", "null");
        {
            std::string many = R"("SCI_Controls":[)";
            for (int i = 0; i < 21; ++i) {
                many += i == 0 ? "\"HCS\"" : ",\"HCS\"";
            }
            many += "]";
            reject("sci_controls 21 > max 20", with(kGolden, owners, owners + "," + many), sec + ".SCI_Controls",
                   "occurrence(s) rejected");
            std::string twenty = R"("SCI_Controls":[)";
            for (int i = 0; i < 20; ++i) {
                twenty += i == 0 ? "\"HCS\"" : ",\"HCS\"";
            }
            twenty += "]";
            const m::LogMT max = decode_ok(with(kGolden, owners, owners + "," + twenty));
            CHECK(max.securityinformation.sci_controls.values().size() == 20);
        }
        reject("scalar for repetition", with(kGolden, owners, owners + R"(,"SCI_Controls":"HCS")"),
               sec + ".SCI_Controls", "expected array");
        reject("bad element in repetition",
               with(kGolden, owners, owners + R"(,"SCI_Controls":["HCS","NOPE"])"), sec + ".SCI_Controls[1]",
               "not a member of enumeration");
        reject("componentid wrong kind", with(kGolden, R"("Severity")", R"("ComponentID":{},"Severity")"),
               "LogMT.MessageData.ComponentID", "expected array");
        reject("componentid bad element",
               with(kGolden, R"("Severity")", R"("ComponentID":[{"UUID":"nope"}],"Severity")"),
               "LogMT.MessageData.ComponentID[0].UUID", "generated model");
        reject("missing SecurityInformation", with(kGolden, kSecurity + ",", ""), "LogMT",
               "missing required member \"SecurityInformation\"");
        reject("missing Classification", with(kGolden, R"("Classification":"U",)", ""), sec,
               "missing required member \"Classification\"");
        reject("missing header Timestamp", with(kGolden, R"("Timestamp":"2026-01-02T03:04:05Z",)", ""),
               "LogMT.MessageHeader", "missing required member \"Timestamp\"");
        reject("missing SchemaVersion", with(kGolden, R"("SchemaVersion":")" + kVersion + R"(",)", ""),
               "LogMT.MessageHeader", "missing required member \"SchemaVersion\"");
        reject("missing Mode", with(kGolden, R"(,"Mode":"LIVE")", ""), "LogMT.MessageHeader",
               "missing required member \"Mode\"");
        reject("missing data Timestamp", with(kGolden, R"("Timestamp":"2026-01-02T03:04:06Z",)", ""),
               "LogMT.MessageData", "missing required member \"Timestamp\"");
        reject("missing Severity", with(kGolden, R"("Severity":"INFO",)", ""), "LogMT.MessageData",
               "missing required member \"Severity\"");
        reject("missing Label", with(kGolden, R"("Label":"Task093",)", ""), "LogMT.MessageData",
               "missing required member \"Label\"");
        reject("missing Details",
               with(kGolden, R"(,"Details":"Independent real Log Binary codec qualification")", ""),
               "LogMT.MessageData", "missing required member \"Details\"");
        reject("unknown top-level member", with(kGolden, R"("MessageHeader")", R"("Extra":1,"MessageHeader")"), "LogMT",
               "unknown member \"Extra\"");
        reject("unknown nested member", with(kGolden, R"("Label":"Task093")", R"("Label":"Task093","Nope":true)"),
               "LogMT.MessageData", "unknown member \"Nope\"");
        reject("global-element envelope", "{\"Log\":" + kGolden + "}", "LogMT", "unknown member \"Log\"");
        reject("wrong $type", with(kGolden, R"("SecurityInformation")", R"("$type":"OtherMT","SecurityInformation")"),
               "LogMT", "$type");
        reject("severity unknown", with(kGolden, R"("Severity":"INFO")", R"("Severity":"CHATTY")"),
               "LogMT.MessageData.Severity", "not a member of enumeration");
        reject("mode unknown", with(kGolden, R"("Mode":"LIVE")", R"("Mode":"live")"), "LogMT.MessageHeader.Mode",
               "not a member of enumeration");
        reject("classification null", with(kGolden, R"("Classification":"U")", R"("Classification":null)"),
               "LogMT.SecurityInformation.Classification", "null");
    }

    // ---- 13. constrained String profiles ---------------------------------------------
    {
        const std::string label = R"("Label":"Task093")";
        const std::string where = "LogMT.MessageData.Label";
        reject("label empty (min 1)", with(kGolden, label, R"("Label":"")"), where, "generated model");
        reject("label tab", with(kGolden, label, R"("Label":"a\tb")"), where, "generated model");
        reject("label newline", with(kGolden, label, R"("Label":"a\nb")"), where, "generated model");
        reject("label non-ASCII", with(kGolden, label, "\"Label\":\"caf\xC3\xA9\""), where, "generated model");
        reject("label DEL", with(kGolden, label, R"("Label":"a\u007fb")"), where, "generated model");
        reject("label NUL", with(kGolden, label, R"("Label":"a\u0000b")"), where, "generated model");
        reject("label 257 characters", with(kGolden, label, "\"Label\":\"" + std::string(257, 'x') + "\""), where,
               "generated model");
        reject("label number", with(kGolden, label, R"("Label":7)"), where, "expected string");
        const m::LogMT max = decode_ok(with(kGolden, label, "\"Label\":\"" + std::string(256, ' ') + "\""));
        CHECK(max.messagedata.label.value() == std::string(256, ' '));  // visible ASCII includes SPACE
        const m::LogMT quoted = decode_ok(with(kGolden, label, R"("Label":"say \"hi\" \\ ~")"));
        CHECK(quoted.messagedata.label.value() == "say \"hi\" \\ ~");
        CHECK(encode_ok(quoted) == with(kGolden, label, R"("Label":"say \"hi\" \\ ~")"));
        const std::string details = R"(,"Details":"Independent real Log Binary codec qualification")";
        reject("details 1025 characters", with(kGolden, details, ",\"Details\":\"" + std::string(1025, 'd') + "\""),
               "LogMT.MessageData.Details", "generated model");
        decode_ok(with(kGolden, details, ",\"Details\":\"" + std::string(1024, 'd') + "\""));
        const std::string version = R"("SchemaVersion":")" + kVersion + "\"";
        const std::string vwhere = "LogMT.MessageHeader.SchemaVersion";
        for (const char* text : {"2.5", "002.5", "002.5.0.0", "002.5.0 ", "", "abc", "002.5.0_", "0025.0"}) {
            reject(std::string("schema version ") + text, with(kGolden, version, std::string(R"("SchemaVersion":")") + text + "\""),
                   vwhere, "generated model");
        }
        // The optional suffixes of the authoritative pattern are accepted.
        decode_ok(with(kGolden, version, R"("SchemaVersion":"002.5.0a_Pre-1")"));
        // Whitespace-visible profile: control characters other than collapsed XML whitespace fail.
        const std::string security_object = "\"SecurityInformation\":{";
        reject("classifiedby control char",
               with(kGolden, security_object, security_object + R"("ClassifiedBy":"a\u0001b",)"),
               "LogMT.SecurityInformation.ClassifiedBy", "generated model");
        const m::LogMT stored = decode_ok(with(kGolden, security_object, security_object + R"("ClassifiedBy":)" + kWsJsonIn + ","));
        CHECK(stored.securityinformation.classifiedby->value() == kWsJsonStored);
        if (kWsCollapses) {
            // minLength 0: the empty string is a valid, present value.
            const m::LogMT empty = decode_ok(with(kGolden, security_object, security_object + R"("ClassifiedBy":"",)"));
            CHECK(empty.securityinformation.classifiedby.has_value());
            CHECK(encode_ok(empty) ==
                  with(kGolden, R"("NATO:EXAMPLE"}]})", R"("NATO:EXAMPLE"}],"ClassifiedBy":""})"));
        } else {
            reject("classifiedby empty (min 1)", with(kGolden, security_object, security_object + R"("ClassifiedBy":"",)"),
                   "LogMT.SecurityInformation.ClassifiedBy", "generated model");
            reject("classifiedby TAB", with(kGolden, security_object, security_object + R"("ClassifiedBy":"a\tb",)"),
                   "LogMT.SecurityInformation.ClassifiedBy", "generated model");
        }
    }

    // ---- 14. JSON-level rejection with `$` paths ----------------------------------------
    {
        reject("empty input", "", "$", "");
        reject("not json", "Log", "$", "");
        reject("trailing content", kGolden + " x", "$", "");
        reject("trailing comma", with(kGolden, R"("Mode":"LIVE")", R"("Mode":"LIVE",)"), "$", "");
        reject("array root", "[]", "LogMT", "expected object");
        reject("null root", "null", "LogMT", "expected object");
        reject("BOM", "\xEF\xBB\xBF" + kGolden, "$", "");
        reject("raw control char in string", with(kGolden, R"("Task093")", "\"Task\x01\""), "$", "");
        reject("lone surrogate escape", with(kGolden, R"("Task093")", R"("\ud800")"), "$", "");
        for (std::size_t cut = 0; cut < kGolden.size(); cut += 7) {
            ++rejections;
            ++checks;
            if (sc::decode_LogMT(kGolden.substr(0, cut)).ok()) {
                ++failures;
                std::cerr << "TRUNCATION ACCEPTED at " << cut << '\n';
            }
        }
        // Single-byte corruption must never crash; any outcome is acceptable.
        std::size_t decoded_after_corruption = 0;
        for (std::size_t i = 0; i < kGolden.size(); ++i) {
            std::string mutated = kGolden;
            mutated[i] = static_cast<char>(mutated[i] ^ 0x20);
            if (sc::decode_LogMT(mutated).ok()) {
                ++decoded_after_corruption;
            }
        }
        CHECK(decoded_after_corruption < kGolden.size());
    }

    // ---- 15. copy semantics survive the codec ----------------------------------------------
    {
        const m::LogMT a = decode_ok(kGolden);
        m::LogMT b = a;
        b.messagedata.data = std::nullopt;
        CHECK(encode_ok(a) == kGolden);
        CHECK(encode_ok(b) != kGolden);
    }

    if (failures != 0) {
        std::cerr << "CPP_OMS_002_LOG_FAILED failures=" << failures << " checks=" << checks << '\n';
        return 1;
    }
    std::cout << "CPP_OMS_002_LOG_COMPLETE uci=" << UCI_LOG_VERSION << " checks=" << checks
              << " rejections=" << rejections << '\n';
    return 0;
}
