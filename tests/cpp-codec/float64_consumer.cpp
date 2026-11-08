// CPP-OMS-004 synthetic Float64 consumer (NOT UCI). Compiled against the
// production-generated model and codec for tests/fixtures/cpp-codec/float64.
//
// Modes:
//   float64_consumer
//       deterministic edge, domain, path, negative-control and bounded-input
//       checks; prints `CPP_OMS_004_FLOAT64 checks=N rejections=M`.
//   float64_consumer decode CONVERSIONS.TSV
//       decodes every `group<TAB>expected<TAB>token` row of the Task 096
//       oracle corpus through the generated decoder (direct and named
//       positions) and compares the binary64 bit pattern with the oracle's
//       expectation (OVERFLOW rows must be rejected as overflow).
//   float64_consumer encode ENCODING.TSV OUT.TSV
//       encodes every bit pattern through the generated encoder and writes
//       `BITS<TAB>TOKEN`; tests/cpp-codec/float64_oracle.py checks each token
//       against the exact-rational oracle (value, shortest, closest, layout).
//   float64_consumer locale
//       repeats a decode/encode subset under a comma-decimal C locale; exits 77
//       when no such locale is installed (reported as SKIPPED, never a pass).
//
// Expected bit patterns below are written independently as hexadecimal IEEE
// binary64 encodings; none is computed by the code under test.
#include <chrono>
#include <clocale>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iostream>
#include <limits>
#include <sstream>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <vector>

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

std::uint64_t bits_of(double value) {
    std::uint64_t bits = 0;
    std::memcpy(&bits, &value, sizeof bits);
    return bits;
}

double from_bits(std::uint64_t bits) {
    double value = 0.0;
    std::memcpy(&value, &bits, sizeof value);
    return value;
}

std::string hex(std::uint64_t bits) {
    static const char digits[] = "0123456789ABCDEF";
    std::string text(16, '0');
    for (int i = 15; i >= 0; --i) {
        text[static_cast<std::size_t>(i)] = digits[bits & 0xFU];
        bits >>= 4U;
    }
    return text;
}

const std::string kRequired = R"("Named":0,"Angle":0)";

/// A minimal document with `member` set to the raw JSON `value`.
std::string doc_with(std::string_view member, std::string_view value) {
    std::string out = "{\"Direct\":0,";
    if (member == "Direct") {
        out = "{\"Direct\":" + std::string(value) + ",";
        out += kRequired;
    } else if (member == "Named") {
        out += "\"Named\":" + std::string(value) + ",\"Angle\":0";
    } else if (member == "Angle") {
        out += "\"Named\":0,\"Angle\":" + std::string(value);
    } else {
        out += kRequired + ",\"" + std::string(member) + "\":" + std::string(value);
    }
    out += '}';
    return out;
}

sc::CodecResult<m::Float64PayloadType> decode(const std::string& json) {
    return sc::decode_Float64PayloadType(json);
}

/// Decode `token` at Direct and require exactly `expected` bits.
void decode_bits(std::string_view token, std::uint64_t expected) {
    const auto decoded = decode(doc_with("Direct", token));
    ++checks;
    if (!decoded.ok()) {
        ++failures;
        std::cerr << "decode " << token.substr(0, 60) << ": rejected: " << decoded.error().path << ": "
                  << decoded.error().message << '\n';
        return;
    }
    if (bits_of(decoded.value().direct) != expected) {
        ++failures;
        std::cerr << "decode " << token.substr(0, 60) << ": bits " << hex(bits_of(decoded.value().direct))
                  << " != expected " << hex(expected) << '\n';
    }
}

/// `json` must be rejected with the path and message fragments.
void reject(const std::string& json, std::string_view path, std::string_view message) {
    const auto decoded = decode(json);
    ++checks;
    ++rejections;
    if (decoded.ok()) {
        ++failures;
        std::cerr << "accepted but must be rejected: " << json.substr(0, 120) << '\n';
        return;
    }
    if (decoded.error().path != path || decoded.error().message.find(message) == std::string::npos) {
        ++failures;
        std::cerr << "wrong error for " << json.substr(0, 120) << "\n  path    " << decoded.error().path
                  << " (expected " << path << ")\n  message " << decoded.error().message << " (expected ~"
                  << message << ")\n";
    }
}

/// Encode a lone value through the Direct member and return its token.
std::string encode_direct(double value) {
    m::Float64PayloadType payload{value,
                                  std::nullopt,
                                  m::UnboundedDoubleType(0.0),
                                  *m::AngleDegreesType::create(0.0),
                                  std::nullopt,
                                  std::nullopt,
                                  std::nullopt,
                                  std::nullopt,
                                  std::nullopt,
                                  std::nullopt,
                                  std::nullopt,
                                  *m::BoundedVector<double, 0, 4>::create({}),
                                  *m::UnboundedVector<m::AngleDegreesType, 0>::create({}),
                                  std::nullopt,
                                  std::nullopt,
                                  std::nullopt};
    const auto encoded = sc::encode_Float64PayloadType(payload);
    if (!encoded.ok()) {
        return "<encode failed: " + encoded.error().message + ">";
    }
    const std::string& text = encoded.value();
    const std::string prefix = "{\"Direct\":";
    const std::string suffix = ",\"Named\":0,\"Angle\":0}";
    if (text.compare(0, prefix.size(), prefix) != 0 || text.size() < prefix.size() + suffix.size() ||
        text.compare(text.size() - suffix.size(), suffix.size(), suffix) != 0) {
        return "<unexpected document: " + text + ">";
    }
    return text.substr(prefix.size(), text.size() - prefix.size() - suffix.size());
}

void encodes_as(double value, std::string_view expected) {
    const std::string token = encode_direct(value);
    ++checks;
    if (token != expected) {
        ++failures;
        std::cerr << "encode " << hex(bits_of(value)) << ": " << token << " != expected " << expected << '\n';
    }
}

/// A carrier whose bytes were overwritten with `value` (bypassing `create`):
/// the encode-side negative control. Defined behavior for a trivially
/// copyable class of exactly one double.
template <typename Carrier>
Carrier forged(double valid, double value) {
    static_assert(std::is_trivially_copyable<Carrier>::value, "carrier must be trivially copyable");
    static_assert(sizeof(Carrier) == sizeof(double), "carrier must hold exactly one double");
    Carrier carrier = *Carrier::create(valid);
    std::memcpy(static_cast<void*>(&carrier), &value, sizeof value);
    return carrier;
}

// ---------------------------------------------------------------------------

void decode_edges() {
    constexpr std::uint64_t kNegZero = 0x8000000000000000ULL;
    decode_bits("0", 0);
    decode_bits("-0", kNegZero);
    decode_bits("-0.0", kNegZero);
    decode_bits("-0e10", kNegZero);
    decode_bits("0e-999", 0);
    decode_bits("-0E+99999999999999999999", kNegZero);
    decode_bits("7", 0x401C000000000000ULL);
    decode_bits("1E2", 0x4059000000000000ULL);
    decode_bits("1e+2", 0x4059000000000000ULL);
    decode_bits("-1.5", 0xBFF8000000000000ULL);
    decode_bits("0.1", 0x3FB999999999999AULL);
    decode_bits("1.0000000000000002", 0x3FF0000000000001ULL);
    decode_bits("0.9999999999999999", 0x3FEFFFFFFFFFFFFFULL);
    // 1 + 2^-53 is the exact midpoint of 1 and its successor: ties to even.
    decode_bits("1.00000000000000011102230246251565404236316680908203125", 0x3FF0000000000000ULL);
    decode_bits("1.00000000000000011102230246251565404236316680908203126", 0x3FF0000000000001ULL);
    decode_bits("1.00000000000000011102230246251565404236316680908203124", 0x3FF0000000000000ULL);
    // 1 + 3 * 2^-53 is the midpoint of the 1st and 2nd successors: ties to even (2nd).
    decode_bits("1.00000000000000033306690738754696212708950042724609375", 0x3FF0000000000002ULL);
    decode_bits("9007199254740993", 0x4340000000000000ULL);
    decode_bits("9007199254740995", 0x4340000000000002ULL);
    decode_bits("-9223372036854775808", 0xC3E0000000000000ULL);
    decode_bits("18446744073709551615", 0x43F0000000000000ULL);
    // Subnormal and normal boundaries.
    decode_bits("4.9406564584124654e-324", 0x0000000000000001ULL);
    decode_bits("5e-324", 0x0000000000000001ULL);
    decode_bits("-5e-324", 0x8000000000000001ULL);
    decode_bits("1e-323", 0x0000000000000002ULL);
    decode_bits("2.2250738585072009e-308", 0x000FFFFFFFFFFFFFULL);
    decode_bits("2.2250738585072014e-308", 0x0010000000000000ULL);
    // Half the smallest subnormal (2^-1075) ties to even, i.e. to zero; just
    // above it rounds up to the smallest subnormal.
    decode_bits("2.4703282292062327208828439643411068618252990130716238221279284125033775363510437593264991818081799618989828234772285886546332835517796989819938739800539093906315035659515570226392290858392449105184435931802849936536152500319370457678249219365623669863658480757001585769269903706311928279558551332927834338409351978015531246597263579574622766465272827220056374006485499977096599470454020828166226237857393450736339007967761930577506740176324673600968951340535537458516661134223766678604162159680461914467291840300530057530849048765391711386591646239524912623653881879636239373280423891018672348497668235089863388587925628302755995657524455507255189313690836254779186948667994968324049705821028513185451396213837722826145437693412532098591327667236328125e-324", 0);
    decode_bits("2.4703282292062327208828439643411068618252990130716238221279284125033775363510437593264991818081799618989828234772285886546332835517796989819938739800539093906315035659515570226392290858392449105184435931802849936536152500319370457678249219365623669863658480757001585769269903706311928279558551332927834338409351978015531246597263579574622766465272827220056374006485499977096599470454020828166226237857393450736339007967761930577506740176324673600968951340535537458516661134223766678604162159680461914467291840300530057530849048765391711386591646239524912623653881879636239373280423891018672348497668235089863388587925628302755995657524455507255189313690836254779186948667994968324049705821028513185451396213837722826145437693412532098591327667236328126e-324", 1);
    decode_bits("2.4703282292062327e-324", 0);
    decode_bits("-2.4703282292062327e-324", kNegZero);
    decode_bits("2.4703282292062328e-324", 1);
    decode_bits("1e-400", 0);
    decode_bits("-1e-400", kNegZero);
    decode_bits("1e-99999999999999999999", 0);
    decode_bits("-1e-99999999999999999999", kNegZero);
    // Largest finite and the overflow threshold (MAX + half an ulp = 2^1024 - 2^970).
    decode_bits("1.7976931348623157e308", 0x7FEFFFFFFFFFFFFFULL);
    decode_bits("-1.7976931348623157e308", 0xFFEFFFFFFFFFFFFFULL);
    decode_bits("1.7976931348623158e308", 0x7FEFFFFFFFFFFFFFULL);
    decode_bits("179769313486231580793728971405303415079934132710037826936173778980444968292764750946649017977587207096330286416692887910946555547851940402630657488671505820681908902000708383676273854845817711531764475730270069855571366959622842914819860834936475292719074168444365510704342711559699508093042880177904174497791", 0x7FEFFFFFFFFFFFFFULL);
    reject(doc_with("Direct", "179769313486231580793728971405303415079934132710037826936173778980444968292764750946649017977587207096330286416692887910946555547851940402630657488671505820681908902000708383676273854845817711531764475730270069855571366959622842914819860834936475292719074168444365510704342711559699508093042880177904174497792"),
           "Float64PayloadType.Direct", "overflows binary64");
    reject(doc_with("Direct", "1.7976931348623159e308"), "Float64PayloadType.Direct", "overflows binary64");
    reject(doc_with("Direct", "-1.7976931348623159e308"), "Float64PayloadType.Direct", "overflows binary64");
    reject(doc_with("Direct", "1e309"), "Float64PayloadType.Direct", "overflows binary64");
    reject(doc_with("Direct", "-1e309"), "Float64PayloadType.Direct", "overflows binary64");
    reject(doc_with("Direct", "1e99999999999999999999"), "Float64PayloadType.Direct", "overflows binary64");
    reject(doc_with("Named", "1e400"), "Float64PayloadType.Named", "overflows binary64");

    // Specials: exactly the three OMS strings, after JSON unescaping.
    {
        const auto nan = decode(doc_with("Direct", "\"NaN\""));
        CHECK(nan.ok() && std::isnan(nan.value().direct));
        const auto escaped = decode(doc_with("Direct", "\"\\u004eaN\""));
        CHECK(escaped.ok() && std::isnan(escaped.value().direct));
        const auto inf = decode(doc_with("Direct", "\"Infinity\""));
        CHECK(inf.ok() && bits_of(inf.value().direct) == 0x7FF0000000000000ULL);
        const auto ninf = decode(doc_with("Direct", "\"-Infinity\""));
        CHECK(ninf.ok() && bits_of(ninf.value().direct) == 0xFFF0000000000000ULL);
        const auto named = decode(doc_with("Named", "\"-Infinity\""));
        CHECK(named.ok() && bits_of(named.value().named.value()) == 0xFFF0000000000000ULL);
    }
    for (const char* spelling : {"\"nan\"", "\"NAN\"", "\"-NaN\"", "\"+NaN\"", "\"inf\"", "\"Inf\"",
                                 "\"infinity\"", "\"INFINITY\"", "\"+Infinity\"", "\"-infinity\"",
                                 "\" NaN\"", "\"NaN \"", "\"Infinity\\u0000\"", "\"\"", "\"1.5\"",
                                 "\"0\"", "\"-0\"", "\"1e3\"", "\"0x1p0\"", "\"1,5\""}) {
        reject(doc_with("Direct", spelling), "Float64PayloadType.Direct", "must be exactly \"NaN\"");
    }
    for (const auto& kind : {std::pair<const char*, const char*>{"true", "found boolean"},
                             {"false", "found boolean"},
                             {"null", "found null"},
                             {"[]", "found array"},
                             {"[1]", "found array"},
                             {"{}", "found object"}}) {
        reject(doc_with("Direct", kind.first), "Float64PayloadType.Direct", kind.second);
    }
    // Not JSON numbers at all: the strict reader rejects them as syntax.
    for (const char* lexeme : {"NaN", "Infinity", "-Infinity", "nan", "inf", "+1", "01", "-01", "1.", ".5",
                               "1e", "1e+", "1E-", "0x10", "--1", "-", "1.e3", "1_0", "١"}) {
        reject(doc_with("Direct", lexeme), "$", "");
    }
}

void domains() {
    const auto accepts = [](std::string_view member, std::string_view value) {
        return decode(doc_with(member, value)).ok();
    };
    const auto rejects = [](std::string_view member, std::string_view value) {
        reject(doc_with(member, value), "Float64PayloadType." + std::string(member),
               "rejected by the generated model's domain check");
    };
    // [-180, 180]
    CHECK(accepts("Angle", "-180"));
    CHECK(accepts("Angle", "180"));
    CHECK(accepts("Angle", "180.0"));
    CHECK(accepts("Angle", "1.8e2"));
    rejects("Angle", "180.00000000000003");
    rejects("Angle", "-180.00000000000003");
    rejects("Angle", "\"NaN\"");
    rejects("Angle", "\"Infinity\"");
    rejects("Angle", "\"-Infinity\"");
    {
        const auto zero = decode(doc_with("Angle", "-0"));
        CHECK(zero.ok() && bits_of(zero.value().angle.value()) == 0x8000000000000000ULL);
    }
    // >= -6378237 (one-sided: +Infinity is inside, -Infinity and NaN are not)
    CHECK(accepts("Altitude", "-6378237"));
    CHECK(accepts("Altitude", "\"Infinity\""));
    CHECK(accepts("Altitude", "1.7976931348623157e308"));
    rejects("Altitude", "-6378237.000000001");
    rejects("Altitude", "\"-Infinity\"");
    rejects("Altitude", "\"NaN\"");
    // > 0 (exclusive: both zero signs are outside; the smallest subnormal is inside)
    rejects("Positive", "0");
    rejects("Positive", "-0");
    rejects("Positive", "1e-400");  // underflows to +0, then the domain rejects it
    CHECK(accepts("Positive", "5e-324"));
    CHECK(accepts("Positive", "2.4703282292062328e-324"));
    CHECK(accepts("Positive", "\"Infinity\""));
    // < 1
    rejects("BelowOne", "1");
    rejects("BelowOne", "1.0000000000000001");  // rounds to 1
    CHECK(accepts("BelowOne", "0.9999999999999999"));
    CHECK(accepts("BelowOne", "\"-Infinity\""));
    rejects("BelowOne", "\"Infinity\"");
    // [-3.5, 3.5)
    CHECK(accepts("Mixed", "-3.5"));
    CHECK(accepts("Mixed", "3.4999999999999996"));
    rejects("Mixed", "3.5");
    rejects("Mixed", "-3.5000000000000004");
    // (0, 1)
    rejects("Open", "0");
    rejects("Open", "1");
    rejects("Open", "\"NaN\"");
    CHECK(accepts("Open", "0.5"));
    CHECK(accepts("Open", "5e-324"));
    CHECK(accepts("Open", "0.9999999999999999"));
    // [min subnormal, min normal)
    CHECK(accepts("Tiny", "5e-324"));
    CHECK(accepts("Tiny", "2.225073858507201e-308"));
    rejects("Tiny", "2.2250738585072014e-308");
    rejects("Tiny", "2.4703282292062327e-324");  // correctly rounds to +0
    rejects("Tiny", "0");
    // restriction chain [0, 10]
    CHECK(accepts("Derived", "10"));
    CHECK(accepts("Derived", "-0"));
    rejects("Derived", "10.000000000000002");
    rejects("Derived", "-5e-324");
    // unconstrained named: every binary64 including the specials
    CHECK(accepts("Named", "\"NaN\""));
    CHECK(accepts("Named", "\"Infinity\""));
    CHECK(accepts("Named", "-1.7976931348623157e308"));

    // Paths into repetition, Choice and nested Records.
    reject(R"({"Direct":0,"Named":0,"Angle":0,"Samples":[1,2,"nan"]})", "Float64PayloadType.Samples[2]",
           "must be exactly \"NaN\"");
    reject(R"({"Direct":0,"Named":0,"Angle":0,"Samples":[1,2,3,4,5]})", "Float64PayloadType.Samples",
           "5 occurrence(s) rejected");
    reject(R"({"Direct":0,"Named":0,"Angle":0,"NamedSamples":[0,181]})", "Float64PayloadType.NamedSamples[1]",
           "domain check");
    reject(R"({"Direct":0,"Named":0,"Angle":0,"Pick":{"Value":1e999}})", "Float64PayloadType.Pick.Value",
           "overflows binary64");
    reject(R"({"Direct":0,"Named":0,"Angle":0,"Pick":{"Angle":"Infinity"}})", "Float64PayloadType.Pick.Angle",
           "domain check");
    reject(R"({"Direct":0,"Named":0,"Angle":0,"Pick":{"Value":1,"Flag":true}})", "Float64PayloadType.Pick",
           "exactly one selected member");
    reject(R"({"Direct":0,"Named":0,"Angle":0,"Inner":{"Value":true}})", "Float64PayloadType.Inner.Value",
           "found boolean");
    reject(R"({"Named":0,"Angle":0})", "Float64PayloadType", "missing required member \"Direct\"");
    reject(R"({"Direct":0,"Direct":1,"Named":0,"Angle":0})", "$", "duplicate object member");
    reject(R"({"Direct":null,"Named":0,"Angle":0})", "Float64PayloadType.Direct", "found null");

    // Integer token semantics are unchanged next to Float64.
    CHECK(accepts("Count", "7"));
    CHECK(accepts("Count", "-0"));
    reject(doc_with("Count", "1.0"), "Float64PayloadType.Count", "expected an integer");
    reject(doc_with("Count", "1e0"), "Float64PayloadType.Count", "expected an integer");
    reject(doc_with("Count", "\"NaN\""), "Float64PayloadType.Count", "expected number");
    reject(doc_with("Count", "2147483648"), "Float64PayloadType.Count", "domain check");
}

void encodes() {
    encodes_as(0.0, "0");
    encodes_as(from_bits(0x8000000000000000ULL), "-0");
    encodes_as(1.0, "1");
    encodes_as(-1.5, "-1.5");
    encodes_as(0.1, "0.1");
    encodes_as(0.30000000000000004, "0.30000000000000004");
    encodes_as(1.0 / 3.0, "0.3333333333333333");
    encodes_as(100.0, "100");
    encodes_as(123.456, "123.456");
    encodes_as(from_bits(0x3FF0000000000001ULL), "1.0000000000000002");
    encodes_as(9007199254740992.0, "9007199254740992");
    encodes_as(1e20, "100000000000000000000");
    encodes_as(123456789012345680000.0, "123456789012345680000");
    encodes_as(12345678901234567890.0, "12345678901234567000");
    encodes_as(1e21, "1e+21");
    encodes_as(1.5e300, "1.5e+300");
    encodes_as(1e-6, "0.000001");
    encodes_as(1.25e-6, "0.00000125");
    encodes_as(1e-7, "1e-7");
    encodes_as(1.5e-7, "1.5e-7");
    encodes_as(-1.5e-7, "-1.5e-7");
    encodes_as(from_bits(0x0000000000000001ULL), "5e-324");
    encodes_as(from_bits(0x8000000000000001ULL), "-5e-324");
    encodes_as(from_bits(0x000FFFFFFFFFFFFFULL), "2.225073858507201e-308");
    encodes_as(from_bits(0x0010000000000000ULL), "2.2250738585072014e-308");
    encodes_as(from_bits(0x7FEFFFFFFFFFFFFFULL), "1.7976931348623157e+308");
    encodes_as(from_bits(0xFFEFFFFFFFFFFFFFULL), "-1.7976931348623157e+308");
    encodes_as(std::numeric_limits<double>::quiet_NaN(), "\"NaN\"");
    encodes_as(from_bits(0xFFF8000000000000ULL), "\"NaN\"");  // negative quiet NaN
    encodes_as(from_bits(0x7FF0000000000001ULL), "\"NaN\"");  // signaling payload
    encodes_as(from_bits(0x7FFFFFFFFFFFFFFFULL), "\"NaN\"");
    encodes_as(std::numeric_limits<double>::infinity(), "\"Infinity\"");
    encodes_as(-std::numeric_limits<double>::infinity(), "\"-Infinity\"");

    // A fully populated value, exact canonical text in schema order.
    const std::string full =
        R"({"Direct":-0,"Optional":"NaN","Named":"-Infinity","Angle":180,"Altitude":"Infinity",)"
        R"("Positive":5e-324,"BelowOne":0.9999999999999999,"Mixed":-3.5,"Open":0.5,)"
        R"("Tiny":2.225073858507201e-308,"Derived":10,"Samples":[1e+21,1e-7,0.000001,-1.5],)"
        R"("NamedSamples":[-180,0.1],"Pick":{"Value":1.7976931348623157e+308},"Inner":{"Value":"Infinity"},)"
        R"("Count":-7})";
    const auto decoded = decode(full);
    CHECK(decoded.ok());
    if (decoded.ok()) {
        const auto& v = decoded.value();
        CHECK(bits_of(v.direct) == 0x8000000000000000ULL);
        CHECK(v.optional && std::isnan(*v.optional));
        CHECK(bits_of(v.named.value()) == 0xFFF0000000000000ULL);
        CHECK(bits_of(v.positive->value()) == 0x0000000000000001ULL);
        CHECK(bits_of(v.belowone->value()) == 0x3FEFFFFFFFFFFFFFULL);
        CHECK(bits_of(v.tiny->value()) == 0x000FFFFFFFFFFFFFULL);
        CHECK(v.samples.values().size() == 4 && bits_of(v.samples.values()[1]) == 0x3E7AD7F29ABCAF48ULL);
        CHECK(v.namedsamples.values().size() == 2 &&
              bits_of(v.namedsamples.values()[1].value()) == 0x3FB999999999999AULL);
        CHECK(v.pick && std::holds_alternative<m::FloatChoiceType::Value>(v.pick->value) &&
              bits_of(std::get<m::FloatChoiceType::Value>(v.pick->value).value) == 0x7FEFFFFFFFFFFFFFULL);
        CHECK(v.inner && std::isinf(v.inner->value) && v.inner->value > 0.0);
        const auto again = sc::encode_Float64PayloadType(v);
        CHECK(again.ok() && again.value() == full);
    }
    // Non-canonical spellings decode to the same bits and re-encode canonically.
    const auto loose = decode(
        R"({ "Angle" : 1.8E+2 , "Named":-1e0, "Direct": 0.00000000000000000000000000000000001e35 })");
    CHECK(loose.ok() && sc::encode_Float64PayloadType(loose.value()).value() ==
                            R"({"Direct":1,"Named":-1,"Angle":180})");

    // Encode-side negative controls: forged bounded carriers never leave.
    const auto refuses = [](const m::Float64PayloadType& value, std::string_view path) {
        const auto encoded = sc::encode_Float64PayloadType(value);
        ++checks;
        ++rejections;
        if (encoded.ok() || encoded.error().path != path ||
            encoded.error().message.find("domain check") == std::string::npos) {
            ++failures;
            std::cerr << "forged carrier at " << path << " was not refused ("
                      << (encoded.ok() ? encoded.value() : encoded.error().path + ": " + encoded.error().message)
                      << ")\n";
        }
    };
    const auto base = [] { return decode(R"({"Direct":0,"Named":0,"Angle":0})").value(); };
    {
        auto v = base();
        v.angle = forged<m::AngleDegreesType>(0.0, 181.0);
        refuses(v, "Float64PayloadType.Angle");
    }
    {
        auto v = base();
        v.angle = forged<m::AngleDegreesType>(0.0, std::numeric_limits<double>::quiet_NaN());
        refuses(v, "Float64PayloadType.Angle");
    }
    {
        auto v = base();
        v.positive = forged<m::PositiveType>(1.0, 0.0);
        refuses(v, "Float64PayloadType.Positive");
    }
    {
        auto v = base();
        v.altitude = forged<m::AltitudeMetersType>(0.0, -std::numeric_limits<double>::infinity());
        refuses(v, "Float64PayloadType.Altitude");
    }
    {
        auto v = base();
        v.open = forged<m::OpenUnitType>(0.5, 1.0);
        refuses(v, "Float64PayloadType.Open");
    }
    {
        auto v = base();
        v.tiny = forged<m::TinyType>(1e-310, 0.0);
        refuses(v, "Float64PayloadType.Tiny");
    }
    {
        auto v = base();
        v.namedsamples = *m::UnboundedVector<m::AngleDegreesType, 0>::create(
            {*m::AngleDegreesType::create(1.0), forged<m::AngleDegreesType>(0.0, -180.00000000000003)});
        refuses(v, "Float64PayloadType.NamedSamples[1]");
    }
    {
        auto v = base();
        v.pick = m::FloatChoiceType{m::FloatChoiceType::Angle{forged<m::AngleDegreesType>(0.0, 200.0)}};
        refuses(v, "Float64PayloadType.Pick.Angle");
    }
    {
        auto v = base();
        v.derived = forged<m::DerivedRateType>(0.0, 10.000000000000002);
        refuses(v, "Float64PayloadType.Derived");
    }
}

/// Very long lexemes: correct results, linear work, no crash.
void bounded_inputs() {
    const std::size_t n = 1000000;
    const auto started = std::chrono::steady_clock::now();
    reject(doc_with("Direct", std::string(n, '1')), "Float64PayloadType.Direct", "overflows binary64");
    decode_bits("0." + std::string(n, '0') + "1", 0);
    decode_bits("-0." + std::string(n, '0') + "1", 0x8000000000000000ULL);
    decode_bits("-0." + std::string(n, '0'), 0x8000000000000000ULL);
    decode_bits("1" + std::string(n, '0') + "e-1000000", 0x3FF0000000000000ULL);
    decode_bits("0." + std::string(n, '0') + "1e1000010", 0x41CDCD6500000000ULL);  // 1e9
    decode_bits("1.5" + std::string(n, '0'), 0x3FF8000000000000ULL);
    // min subnormal / 2 followed by a long nonzero tail: strictly above the tie.
    decode_bits("2.4703282292062327208828439643411068618252990130716238221279284125033775363510437593264991818081799618989828234772285886546332835517796989819938739800539093906315035659515570226392290858392449105184435931802849936536152500319370457678249219365623669863658480757001585769269903706311928279558551332927834338409351978015531246597263579574622766465272827220056374006485499977096599470454020828166226237857393450736339007967761930577506740176324673600968951340535537458516661134223766678604162159680461914467291840300530057530849048765391711386591646239524912623653881879636239373280423891018672348497668235089863388587925628302755995657524455507255189313690836254779186948667994968324049705821028513185451396213837722826145437693412532098591327667236328125" +
                    std::string(n, '0') + "1e-324",
                1);
    reject(doc_with("Direct", "1e" + std::string(n, '9')), "Float64PayloadType.Direct", "overflows binary64");
    decode_bits("1e-" + std::string(n, '9'), 0);
    decode_bits("0e" + std::string(n, '9'), 0);
    reject(doc_with("Direct", "\"" + std::string(n, 'N') + "\""), "Float64PayloadType.Direct",
           "must be exactly \"NaN\"");
    reject(doc_with("Direct", std::string(n, '1') + "."), "$", "invalid number fraction");
    // Single-byte corruptions of a Float64 document must never crash.
    const std::string sample = R"({"Direct":-1.2345e-300,"Named":"NaN","Angle":-0.5,"Samples":[1e21,5e-324]})";
    for (std::size_t i = 0; i < sample.size(); ++i) {
        for (const char replacement : {'\0', '"', '-', '.', 'e', '9', '+', ' ', 'N', '}'}) {
            std::string corrupt = sample;
            corrupt[i] = replacement;
            static_cast<void>(decode(corrupt));
        }
    }
    for (std::size_t length = 0; length < sample.size(); ++length) {
        CHECK(!decode(sample.substr(0, length)).ok());
    }
    const auto elapsed = std::chrono::steady_clock::now() - started;
    // Generous bound (sanitizer builds included): linear work over ~10 MB.
    CHECK(elapsed < std::chrono::seconds(60));
}

// ---------------------------------------------------------------------------

int run_decode(const char* path) {
    std::ifstream in(path);
    if (!in) {
        std::cerr << "cannot read " << path << '\n';
        return 2;
    }
    std::string line;
    std::size_t rows = 0, finite = 0, overflow = 0, mismatches = 0;
    while (std::getline(in, line)) {
        const std::size_t a = line.find('\t');
        const std::size_t b = line.find('\t', a + 1);
        if (a == std::string::npos || b == std::string::npos) {
            std::cerr << "malformed row " << rows << '\n';
            return 2;
        }
        const std::string expected = line.substr(a + 1, b - a - 1);
        const std::string token = line.substr(b + 1);
        ++rows;
        for (const char* member : {"Direct", "Named"}) {
            const auto decoded = decode(doc_with(member, token));
            if (expected == "OVERFLOW") {
                if (decoded.ok() || decoded.error().message.find("overflows binary64") == std::string::npos) {
                    ++mismatches;
                    std::cerr << "row " << rows << " " << member << ": overflow not rejected: "
                              << token.substr(0, 60) << '\n';
                }
                continue;
            }
            if (!decoded.ok()) {
                ++mismatches;
                std::cerr << "row " << rows << " " << member << ": rejected " << token.substr(0, 60) << ": "
                          << decoded.error().message << '\n';
                continue;
            }
            const double value =
                std::string_view(member) == "Direct" ? decoded.value().direct : decoded.value().named.value();
            if (hex(bits_of(value)) != expected) {
                ++mismatches;
                std::cerr << "row " << rows << " " << member << ": " << token.substr(0, 60) << " -> "
                          << hex(bits_of(value)) << " expected " << expected << '\n';
            }
        }
        (expected == "OVERFLOW" ? overflow : finite) += 1;
    }
    std::cout << "CPP_OMS_004_DECODE rows=" << rows << " finite=" << finite << " overflow=" << overflow
              << " mismatches=" << mismatches << '\n';
    return mismatches == 0 && rows > 0 ? 0 : 1;
}

int run_encode(const char* input, const char* output) {
    std::ifstream in(input);
    std::ofstream out(output);
    if (!in || !out) {
        std::cerr << "cannot open " << input << " / " << output << '\n';
        return 2;
    }
    std::string line;
    std::size_t rows = 0, roundtrip = 0;
    while (std::getline(in, line)) {
        if (line.size() != 16) {
            std::cerr << "malformed row " << rows << '\n';
            return 2;
        }
        const std::uint64_t bits = std::stoull(line, nullptr, 16);
        const std::string token = encode_direct(from_bits(bits));
        out << line << '\t' << token << '\n';
        ++rows;
        // Secondary self-consistency only (the oracle is the authority).
        const auto back = decode(doc_with("Direct", token));
        if (!back.ok() || bits_of(back.value().direct) != bits) {
            ++roundtrip;
        }
    }
    std::cout << "CPP_OMS_004_ENCODE rows=" << rows << " roundtrip_mismatches=" << roundtrip << '\n';
    return roundtrip == 0 && rows > 0 ? 0 : 1;
}

int run_locale() {
    const char* chosen = nullptr;
    for (const char* name : {"de_DE.UTF-8", "de_DE.utf8", "fr_FR.UTF-8", "fr_FR.utf8", "ru_RU.UTF-8"}) {
        if (std::setlocale(LC_ALL, name) != nullptr) {
            chosen = name;
            break;
        }
    }
    if (chosen == nullptr) {
        std::cout << "CPP_OMS_004_LOCALE SKIPPED (no comma-decimal locale installed)\n";
        return 77;
    }
    // The locale really is active: the C library now uses a decimal comma.
    CHECK(std::string_view(std::localeconv()->decimal_point) == ",");
    decode_bits("0.1", 0x3FB999999999999AULL);
    decode_bits("-2.5e-3", 0xBF647AE147AE147BULL);
    encodes_as(0.1, "0.1");
    encodes_as(-1.5e300, "-1.5e+300");
    std::cout << "CPP_OMS_004_LOCALE locale=" << chosen << " checks=" << checks << " failures=" << failures
              << '\n';
    return failures == 0 ? 0 : 1;
}

}  // namespace

int main(int argc, char** argv) {
    if (argc == 3 && std::string_view(argv[1]) == "decode") {
        return run_decode(argv[2]);
    }
    if (argc == 4 && std::string_view(argv[1]) == "encode") {
        return run_encode(argv[2], argv[3]);
    }
    if (argc == 2 && std::string_view(argv[1]) == "locale") {
        return run_locale();
    }
    if (argc != 1) {
        std::cerr << "usage: float64_consumer [decode TSV | encode TSV OUT | locale]\n";
        return 2;
    }
    decode_edges();
    domains();
    encodes();
    bounded_inputs();
    if (failures != 0) {
        std::cerr << failures << " failure(s)\n";
        return 1;
    }
    std::cout << "CPP_OMS_004_FLOAT64 checks=" << checks << " rejections=" << rejections << '\n';
    return 0;
}
