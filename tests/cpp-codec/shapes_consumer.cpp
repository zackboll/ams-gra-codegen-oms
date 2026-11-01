// CPP-OMS-002 synthetic shape consumer (NOT UCI; the authentic Log evidence is
// tests/cpp-codec/log_consumer.cpp). Covers the new value kinds in positions
// the Log closure does not use: bounded and unbounded repetition of hexBinary,
// dateTime and duration, a named hexBinary restriction chain, and a Choice with
// a reserved-word member (`begin`), a nested Choice and a Choice payload.
//
//   -DSHAPE_HEX | -DSHAPE_TEMPORAL | -DSHAPE_CHOICE
//
// The Choice documents are the independently authored Task 088 Ada fixture
// (tests/fixtures/service-generate/codec-ada-choice.json), reproduced verbatim
// in compact form.
#include <iostream>
#include <string>
#include <string_view>
#include <variant>
#include <vector>

#include "oam.hpp"
#include "service_api.hpp"
#include "service_codec.hpp"

namespace m = ::programs::oam;
namespace sc = ::service_codec;

namespace {

int failures = 0;
int checks = 0;

#define CHECK(condition)                                                   \
    do {                                                                   \
        ++checks;                                                          \
        if (!(condition)) {                                                \
            ++failures;                                                    \
            std::cerr << __FILE__ << ':' << __LINE__                       \
                      << ": CHECK failed: " << #condition << '\n';         \
        }                                                                  \
    } while (false)

/// decode(json) must succeed and encode back to exactly `json`.
template <typename Decode, typename Encode>
void round_trip(const char* name, const std::string& json, Decode decode, Encode encode) {
    const auto decoded = decode(json);
    ++checks;
    if (!decoded.ok()) {
        ++failures;
        std::cerr << name << ": decode failed at " << decoded.error().path << ": "
                  << decoded.error().message << '\n';
        return;
    }
    const auto again = encode(decoded.value());
    ++checks;
    if (!again.ok() || again.value() != json) {
        ++failures;
        std::cerr << name << ": re-encode differs\n  expected " << json << "\n  actual   "
                  << (again.ok() ? again.value() : again.error().message) << '\n';
    }
}

template <typename Decode>
void reject(const char* name, const std::string& json, std::string_view path, std::string_view message,
            Decode decode) {
    const auto decoded = decode(json);
    ++checks;
    if (decoded.ok()) {
        ++failures;
        std::cerr << name << ": accepted but must be rejected\n";
        return;
    }
    if (decoded.error().path.find(path) == std::string::npos ||
        decoded.error().message.find(message) == std::string::npos) {
        ++failures;
        std::cerr << name << ": wrong error\n  path    " << decoded.error().path << "\n  message "
                  << decoded.error().message << '\n';
    }
}

}  // namespace

int main() {
#if defined(SHAPE_HEX)
    const auto decode = [](const std::string& json) { return sc::decode_BlobPayload(json); };
    const auto encode = [](const m::BlobPayload& v) { return sc::encode_BlobPayload(v); };
    const std::string full =
        R"({"Id":7,"Data":"00FF","MaybeData":"","Chunks":["AA","BBCC"],"Stream":["01","","0203"],)"
        R"("Blob":"DEADBEEF","Alias":"CAFE","Atomic":{"HexBinaryValue":"0A0B"}})";
    round_trip("hex full", full, decode, encode);
    const std::string minimal = R"({"Id":1,"Data":"","Blob":"00","Atomic":{"IntValue":-5}})";
    round_trip("hex minimal", minimal, decode, encode);
    const auto typed = decode(full);
    CHECK(typed.ok());
    CHECK(typed.value().data == (std::vector<std::uint8_t>{0x00, 0xFF}));
    CHECK(typed.value().maybedata.has_value() && typed.value().maybedata->empty());
    CHECK(typed.value().chunks.values().size() == 2);
    CHECK(typed.value().stream.values().size() == 3);
    CHECK(typed.value().stream.values()[1].empty());
    CHECK(typed.value().blob.value() == (std::vector<std::uint8_t>{0xDE, 0xAD, 0xBE, 0xEF}));
    CHECK(typed.value().alias->value() == (std::vector<std::uint8_t>{0xCA, 0xFE}));
    CHECK(std::holds_alternative<m::AtomicLike::HexBinaryValue>(typed.value().atomic.value));
    CHECK(!decode(minimal).value().maybedata.has_value());
    // Lowercase input is canonicalised to uppercase on output.
    CHECK(encode(decode(R"({"Id":1,"Data":"abcdef","Blob":"00","Atomic":{"IntValue":1}})").value()).value() ==
          R"({"Id":1,"Data":"ABCDEF","Blob":"00","Atomic":{"IntValue":1}})");
    reject("hex odd", R"({"Id":1,"Data":"0","Blob":"00","Atomic":{"IntValue":1}})", "BlobPayload.Data", "odd number", decode);
    reject("hex bad digit in repetition",
           R"({"Id":1,"Data":"00","Chunks":["AA","XY"],"Blob":"00","Atomic":{"IntValue":1}})", "Chunks[1]",
           "hexadecimal digits", decode);
    reject("hex too many chunks",
           R"({"Id":1,"Data":"00","Chunks":["AA","BB","CC","DD"],"Blob":"00","Atomic":{"IntValue":1}})", "Chunks",
           "occurrence(s) rejected", decode);
    reject("hex named bad", R"({"Id":1,"Data":"00","Blob":"0Z","Atomic":{"IntValue":1}})", "BlobPayload.Blob",
           "hexadecimal digits", decode);
    reject("hex alias odd", R"({"Id":1,"Data":"00","Blob":"00","Alias":"ABC","Atomic":{"IntValue":1}})",
           "BlobPayload.Alias", "odd number", decode);
    reject("hex choice alternative bad", R"({"Id":1,"Data":"00","Blob":"00","Atomic":{"HexBinaryValue":"G0"}})",
           "Atomic.HexBinaryValue", "hexadecimal digits", decode);
    reject("hex choice empty", R"({"Id":1,"Data":"00","Blob":"00","Atomic":{}})", "BlobPayload.Atomic",
           "exactly one selected member", decode);
    reject("hex choice two",
           R"({"Id":1,"Data":"00","Blob":"00","Atomic":{"IntValue":1,"HexBinaryValue":"00"}})", "BlobPayload.Atomic",
           "exactly one selected member", decode);
#elif defined(SHAPE_TEMPORAL)
    const auto decode = [](const std::string& json) { return sc::decode_TemporalPayload(json); };
    const auto encode = [](const m::TemporalPayload& v) { return sc::encode_TemporalPayload(v); };
    const std::string full =
        R"({"DirectDates":["2026-01-02T03:04:05+01:00","2026-01-02T03:04:05"],"MoreDirectDates":["2026-01-02T03:04:05Z"],)"
        R"("NamedDates":["2026-01-02T03:04:05Z"],"MoreNamedDates":["2026-01-02T03:04:05Z","2026-12-31T23:59:59.5Z"],)"
        R"("DirectSpans":["PT1S"],"MoreDirectSpans":["P1D","-PT0.5S"],"NamedSpans":["P1Y","PT0S"],"MoreNamedSpans":["PT1M"]})";
    round_trip("temporal full", full, decode, encode);
    const std::string minimal =
        R"({"NamedDates":["2026-01-02T03:04:05Z"],"MoreNamedDates":["2026-01-02T03:04:05Z","2026-01-02T03:04:06Z"],)"
        R"("DirectSpans":["PT1S"],"MoreDirectSpans":["PT1S","PT2S"]})";
    round_trip("temporal minimal", minimal, decode, encode);
    const auto typed = decode(full);
    CHECK(typed.ok());
    CHECK(typed.value().directdates.values().size() == 2);
    CHECK(typed.value().directdates.values()[0].value() == "2026-01-02T03:04:05+01:00");
    CHECK(typed.value().moredirectspans.values()[1].value() == "-PT0.5S");
    CHECK(typed.value().nameddates.values()[0].value() == "2026-01-02T03:04:05Z");
    reject("temporal named date needs Z",
           R"({"NamedDates":["2026-01-02T03:04:05+01:00"],"MoreNamedDates":["2026-01-02T03:04:05Z","2026-01-02T03:04:06Z"],"DirectSpans":["PT1S"],"MoreDirectSpans":["PT1S","PT2S"]})",
           "NamedDates[0]", "generated model", decode);
    reject("temporal bad direct date in repetition",
           R"({"DirectDates":["2026-01-02T03:04:05Z","2026-02-30T03:04:05Z"],"NamedDates":["2026-01-02T03:04:05Z"],"MoreNamedDates":["2026-01-02T03:04:05Z","2026-01-02T03:04:06Z"],"DirectSpans":["PT1S"],"MoreDirectSpans":["PT1S","PT2S"]})",
           "DirectDates[1]", "generated model", decode);
    reject("temporal bad span",
           R"({"NamedDates":["2026-01-02T03:04:05Z"],"MoreNamedDates":["2026-01-02T03:04:05Z","2026-01-02T03:04:06Z"],"DirectSpans":["PT"],"MoreDirectSpans":["PT1S","PT2S"]})",
           "DirectSpans[0]", "generated model", decode);
    reject("temporal too few named dates (min 2)",
           R"({"NamedDates":["2026-01-02T03:04:05Z"],"MoreNamedDates":["2026-01-02T03:04:05Z"],"DirectSpans":["PT1S"],"MoreDirectSpans":["PT1S","PT2S"]})",
           "MoreNamedDates", "occurrence(s) rejected", decode);
    reject("temporal missing required span", R"({"NamedDates":["2026-01-02T03:04:05Z"],"MoreNamedDates":["2026-01-02T03:04:05Z","2026-01-02T03:04:06Z"],"MoreDirectSpans":["PT1S","PT2S"]})",
           "DirectSpans", "occurrence(s) rejected", decode);
    reject("temporal span wrong kind",
           R"({"NamedDates":["2026-01-02T03:04:05Z"],"MoreNamedDates":["2026-01-02T03:04:05Z","2026-01-02T03:04:06Z"],"DirectSpans":[1],"MoreDirectSpans":["PT1S","PT2S"]})",
           "DirectSpans[0]", "expected string", decode);
#elif defined(SHAPE_CHOICE)
    const auto decode = [](const std::string& json) { return sc::decode_Selection(json); };
    const auto encode = [](const m::Selection& v) { return sc::encode_Selection(v); };
    const char* const documents[] = {
        R"({"Enabled":true})",          R"({"Number":-7})",
        R"({"begin":"text"})",          R"({"Signal":"5G"})",
        R"({"Checked":"checked"})",     R"({"Nested":{"Text":"nested"}})",
        R"({"Other":{"Flag":false}})",
    };
    for (const char* document : documents) {
        round_trip(document, document, decode, encode);
    }
    // Reserved-word alternative: model name `Begin`, wire name `begin`.
    const auto begin = decode(R"({"begin":"text"})");
    CHECK(begin.ok());
    CHECK(std::holds_alternative<m::Selection::Begin>(begin.value().value));
    CHECK(std::get<m::Selection::Begin>(begin.value().value).value == "text");
    const auto signal = decode(R"({"Signal":"5G"})");
    CHECK(std::get<m::Selection::Signal>(signal.value().value).value == m::Code::Value5G);
    const auto other = decode(R"({"Other":{"Flag":false}})");
    CHECK(std::get<m::Inner::Flag>(std::get<m::Selection::Other>(other.value().value).value.value).value == false);
    const auto payload_decode = [](const std::string& json) { return sc::decode_ChoicePayload(json); };
    const auto payload_encode = [](const m::ChoicePayload& v) { return sc::encode_ChoicePayload(v); };
    round_trip("payload pick only", R"({"Pick":{"Number":3}})", payload_decode, payload_encode);
    round_trip("payload pick and maybe", R"({"Pick":{"Enabled":false},"Maybe":{"Nested":{"Text":"n"}}})",
               payload_decode, payload_encode);
    reject("choice empty", "{}", "Selection", "exactly one selected member", decode);
    reject("choice two", R"({"Enabled":true,"Number":1})", "Selection", "exactly one selected member", decode);
    reject("choice unknown", R"({"Enabled2":true})", "Selection", "unknown member", decode);
    reject("choice bool wrong kind", R"({"Enabled":"true"})", "Selection.Enabled", "expected boolean", decode);
    reject("choice number out of range", R"({"Number":2147483648})", "Selection.Number", "generated model", decode);
    reject("choice enum", R"({"Signal":"6G"})", "Selection.Signal", "not a member of enumeration", decode);
    reject("choice string profile", R"({"Checked":""})", "Selection.Checked", "generated model", decode);
    reject("choice nested empty", R"({"Other":{}})", "Selection.Other", "exactly one selected member", decode);
    reject("choice nested duplicate", R"({"Other":{"Flag":true,"Flag":false}})", "$", "duplicate object member", decode);
    reject("choice array", "[]", "Selection", "expected object", decode);
    reject("choice null", "null", "Selection", "expected object", decode);
    reject("choice payload missing pick", "{}", "ChoicePayload", "missing required member \"Pick\"", payload_decode);
    reject("choice payload bad maybe", R"({"Pick":{"Enabled":true},"Maybe":{}})", "ChoicePayload.Maybe",
           "exactly one selected member", payload_decode);
#else
#error "define SHAPE_HEX, SHAPE_TEMPORAL or SHAPE_CHOICE"
#endif
    if (failures != 0) {
        std::cerr << "CPP_OMS_002_SHAPES_FAILED failures=" << failures << " checks=" << checks << '\n';
        return 1;
    }
    std::cout << "CPP_OMS_002_SHAPES_COMPLETE checks=" << checks << '\n';
    return 0;
}
