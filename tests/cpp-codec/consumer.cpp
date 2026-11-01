// CPP-OMS-001 consumer: compiled against the GENERATED model and service_codec
// headers for tests/fixtures/cpp-codec/{oam.xsd,service.yaml}.
//
// Built by crates/cli/tests/cpp_codec.rs (and scripts/check-cpp-oms-001.sh)
// with strict C++17 flags and run; exit status 0 means every check passed.
#include <cstdint>
#include <cstdlib>
#include <iostream>
#include <limits>
#include <optional>
#include <string>
#include <string_view>
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

using I64 = m::BoundedInteger<std::int64_t, std::numeric_limits<std::int64_t>::min(),
                              9223372036854775807>;
using I16 = m::BoundedInteger<std::int64_t, -32768, 32767>;
using U32 = m::BoundedInteger<std::uint64_t, 0, 4294967295>;
using U8 = m::BoundedInteger<std::uint64_t, 0, 255>;

/// A generated factory that must accept its argument (test setup only).
template <typename T, typename V>
T must(V value) {
    auto created = T::create(std::move(value));
    if (!created) {
        std::cerr << "test setup: a generated factory rejected its argument\n";
        std::exit(2);
    }
    return *created;
}

m::Level level(std::int64_t v) { return must<m::Level>(v); }
U8 u8(std::uint64_t v) { return must<U8>(v); }
U32 u32(std::uint64_t v) { return must<U32>(v); }

// "café "q" \ <LF> U+0001 U+1F600" as stored (UTF-8 bytes, octal escapes).
const std::string kLabel = "caf\303\251 \"q\" \\ \n\001 \360\237\230\200";

// The exact canonical encoding of make_payload().
const std::string kCanonical =
    R"JSON({"class":true,"Enabled":false,"Level":-10,"Wide":4294967295,)JSON"
    R"JSON("Offset":-9223372036854775808,"Big":4294967295,"Small":-32768,)JSON"
    R"JSON("Signal":"5G","Child":{"Label":"café \"q\" \\ \n\u0001 😀","Count":255},)JSON"
    R"JSON("Identity":{"Key":7,"Name":"n","Version":-3},"Empty":{},"Note":"",)JSON"
    R"JSON("Levels":[1,2],"Flags":[true,false,true],"Children":[{"Label":"a","Count":0}],)JSON"
    R"JSON("Signals":["NORMAL","Self"],"Names":["x","y"],"Ids":[0,1,4294967295]})JSON";

// The exact canonical encoding of make_minimal().
const std::string kMinimal =
    R"JSON({"class":false,"Enabled":true,"Level":0,"Wide":0,"Offset":0,"Big":0,"Small":0,)JSON"
    R"JSON("Signal":"Self","Child":{"Label":"","Count":0},"Identity":{"Key":0},)JSON"
    R"JSON("Levels":[0,0],"Names":["only"]})JSON";

m::Payload make_payload() {
    return m::Payload{
        m::Flag(true),
        false,
        level(-10),
        must<m::Wide>(std::uint64_t{4294967295U}),
        must<I64>(std::numeric_limits<std::int64_t>::min()),
        u32(4294967295U),
        must<I16>(std::int64_t{-32768}),
        m::SignalCode::Value5G,
        m::Child{kLabel, u8(255)},
        m::VersionedIdentity{u32(7), std::string("n"), must<I16>(std::int64_t{-3})},
        std::optional<m::Empty>(m::Empty{}),
        std::optional<std::string>(std::string()),
        must<m::BoundedVector<m::Level, 2, 3>>(std::vector<m::Level>{level(1), level(2)}),
        must<m::BoundedVector<bool, 0, 3>>(std::vector<bool>{true, false, true}),
        must<m::BoundedVector<m::Child, 0, 2>>(std::vector<m::Child>{m::Child{"a", u8(0)}}),
        must<m::BoundedVector<m::SignalCode, 0, 4>>(
            std::vector<m::SignalCode>{m::SignalCode::NORMAL, m::SignalCode::Self}),
        must<m::UnboundedVector<std::string, 1>>(std::vector<std::string>{"x", "y"}),
        must<m::UnboundedVector<U32, 0>>(std::vector<U32>{u32(0), u32(1), u32(4294967295U)}),
    };
}

m::Payload make_minimal() {
    return m::Payload{
        m::Flag(false),
        true,
        level(0),
        must<m::Wide>(std::uint64_t{0}),
        must<I64>(std::int64_t{0}),
        u32(0),
        must<I16>(std::int64_t{0}),
        m::SignalCode::Self,
        m::Child{"", u8(0)},
        m::VersionedIdentity{u32(0), std::nullopt, std::nullopt},
        std::nullopt,
        std::nullopt,
        must<m::BoundedVector<m::Level, 2, 3>>(std::vector<m::Level>{level(0), level(0)}),
        must<m::BoundedVector<bool, 0, 3>>(std::vector<bool>{}),
        must<m::BoundedVector<m::Child, 0, 2>>(std::vector<m::Child>{}),
        must<m::BoundedVector<m::SignalCode, 0, 4>>(std::vector<m::SignalCode>{}),
        must<m::UnboundedVector<std::string, 1>>(std::vector<std::string>{"only"}),
        must<m::UnboundedVector<U32, 0>>(std::vector<U32>{}),
    };
}

/// Replace the single occurrence of `from` in `text`.
std::string mutate(const std::string& text, const std::string& from, const std::string& to) {
    const std::size_t at = text.find(from);
    if (at == std::string::npos || text.find(from, at + 1) != std::string::npos) {
        std::cerr << "test setup: '" << from << "' is not unique in the base text\n";
        std::exit(2);
    }
    std::string result = text;
    result.replace(at, from.size(), to);
    return result;
}

/// Decode must fail with exactly this path and a message containing `part`.
void rejects(const std::string& label, const std::string& json, const std::string& path,
             const std::string& part) {
    const auto result = sc::decode_Payload(json);
    ++checks;
    if (result.ok()) {
        ++failures;
        std::cerr << "NOT REJECTED: " << label << '\n';
        return;
    }
    if (result.error().path != path || result.error().message.find(part) == std::string::npos) {
        ++failures;
        std::cerr << "WRONG ERROR for " << label << ": path='" << result.error().path
                  << "' message='" << result.error().message << "' (wanted path='" << path
                  << "' containing '" << part << "')\n";
    }
}

void syntax(const std::string& label, const std::string& json, const std::string& part) {
    rejects(label, json, "$", part);
}

std::string flipped(const std::string& text, std::size_t at, char replacement) {
    std::string copy = text;
    copy[at] = replacement;
    return copy;
}

void check_round_trip_and_exact_wire() {
    const auto encoded = sc::encode_Payload(make_payload());
    CHECK(encoded.ok());
    CHECK(encoded.value() == kCanonical);

    const auto decoded = sc::decode_Payload(kCanonical);
    CHECK(decoded.ok());
    if (decoded.ok()) {
        const m::Payload& v = decoded.value();
        CHECK(v.field_class.value());
        CHECK(!v.enabled);
        CHECK(v.level.value() == -10);
        CHECK(v.wide.value() == 4294967295U);
        CHECK(v.offset.value() == std::numeric_limits<std::int64_t>::min());
        CHECK(v.small.value() == -32768);
        CHECK(v.signal == m::SignalCode::Value5G);
        CHECK(v.child.label == kLabel);
        CHECK(v.child.count.value() == 255);
        CHECK(v.identity.key.value() == 7);
        CHECK(v.identity.name.has_value() && *v.identity.name == "n");
        CHECK(v.identity.version.has_value() && v.identity.version->value() == -3);
        CHECK(v.empty.has_value());
        CHECK(v.note.has_value() && v.note->empty());
        CHECK(v.levels.values().size() == 2);
        CHECK(v.flags.values().size() == 3 && v.flags.values()[0] && !v.flags.values()[1]);
        CHECK(v.children.values().size() == 1 && v.children.values()[0].label == "a");
        CHECK(v.signals.values().size() == 2 && v.signals.values()[1] == m::SignalCode::Self);
        CHECK(v.names.values().size() == 2 && v.names.values()[1] == "y");
        CHECK(v.ids.values().size() == 3 && v.ids.values()[2].value() == 4294967295U);
        const auto again = sc::encode_Payload(v);
        CHECK(again.ok() && again.value() == kCanonical);
    }

    const auto minimal = sc::encode_Payload(make_minimal());
    CHECK(minimal.ok() && minimal.value() == kMinimal);
    const auto minimal_back = sc::decode_Payload(kMinimal);
    CHECK(minimal_back.ok());
    if (minimal_back.ok()) {
        const m::Payload& v = minimal_back.value();
        // Absence is preserved: optional members stay absent, and an absent
        // optional is distinguishable from a present empty String / Record.
        CHECK(!v.empty.has_value() && !v.note.has_value());
        CHECK(!v.identity.name.has_value() && !v.identity.version.has_value());
        CHECK(v.flags.values().empty() && v.ids.values().empty());
        CHECK(sc::encode_Payload(v).value() == kMinimal);
    }

    // Shared payload type: exactly one pair serves both messages; a second
    // payload type has its own pair.
    const auto ack = sc::decode_Ack(R"({"Accepted":true})");
    CHECK(ack.ok() && ack.value().accepted);
    CHECK(sc::encode_Ack(m::Ack{false}).value() == R"({"Accepted":false})");

    // The service API header and the codec header coexist in one unit.
    CHECK(service_api::service_name == "Cpp Codec");
}

void check_non_canonical_input_is_accepted() {
    // Whitespace, member order and \u escapes (including a surrogate pair) are
    // all insignificant; the canonical re-encoding is unchanged.
    const std::string noisy =
        " \t\r\n{ \"Names\" : [ \"x\" , \"y\" ] ,\n \"Ids\":[0,1,4294967295], "
        "\"Signals\":[\"NORMAL\",\"Self\"],\"Children\":[{\"Count\":0,\"Label\":\"a\"}],"
        "\"Flags\":[true,false,true],\"Levels\":[1,2],\"Note\":\"\",\"Empty\":{},"
        "\"Identity\":{\"Version\":-3,\"Name\":\"n\",\"Key\":7},"
        "\"Child\":{\"Count\":255,\"Label\":\"caf\\u00e9 \\\"q\\\" \\\\ \\n\\u0001 \\ud83d\\ude00\"},"
        "\"Signal\":\"5G\",\"Small\":-32768,\"Big\":4294967295,"
        "\"Offset\":-9223372036854775808,\"Wide\":4294967295,\"Level\":-10,"
        "\"Enabled\":false,\"class\":true} \n";
    const auto decoded = sc::decode_Payload(noisy);
    CHECK(decoded.ok());
    if (decoded.ok()) {
        CHECK(sc::encode_Payload(decoded.value()).value() == kCanonical);
    }
    // "-0" is the integer zero; `[]` and absence are the same zero occurrences.
    const auto zero = sc::decode_Payload(mutate(kMinimal, "\"Level\":0", "\"Level\":-0"));
    CHECK(zero.ok() && zero.value().level.value() == 0);
    const auto empty_array = sc::decode_Payload(mutate(kMinimal, "\"Levels\"", "\"Flags\":[],\"Levels\""));
    CHECK(empty_array.ok());
    if (empty_array.ok()) {
        CHECK(sc::encode_Payload(empty_array.value()).value() == kMinimal);
    }
    // "$type" is accepted only when it names the concrete type itself.
    CHECK(sc::decode_Payload(mutate(kMinimal, "{\"class\"", "{\"$type\":\"Payload\",\"class\"")).ok());
    CHECK(sc::decode_Payload(mutate(kMinimal, "\"Child\":{", "\"Child\":{\"$type\":\"Child\",")).ok());
    // An escaped slash and an escaped member-looking string are plain data.
    CHECK(sc::decode_Payload(mutate(kMinimal, "\"only\"", "\"a\\/b\\u0000c\"")).ok());
    // A Boolean member and unknown-looking data inside a String are data.
    CHECK(sc::decode_Payload(mutate(kMinimal, "\"only\"", "\"{\\\"class\\\":1}\"")).ok());
}

void check_syntax_rejections() {
    syntax("empty input", "", "unexpected end of input");
    syntax("only whitespace", " \n", "unexpected end of input");
    syntax("truncated object", "{", "expected a member name string");
    syntax("truncated after member", "{\"a\":1", "unterminated object");
    syntax("trailing comma", mutate(kMinimal, "\"Enabled\":true,", "\"Enabled\":true,,"), "expected a member name");
    syntax("trailing comma before brace", mutate(kMinimal, "[\"only\"]}", "[\"only\"],}"), "expected a member name");
    syntax("trailing array comma", mutate(kMinimal, "[0,0]", "[0,0,]"), "unexpected character");
    syntax("trailing content", kMinimal + "x", "unexpected content after the JSON value");
    syntax("two documents", kMinimal + kMinimal, "unexpected content after the JSON value");
    syntax("byte order mark", "\357\273\277" + kMinimal, "unexpected character");
    syntax("leading zero", mutate(kMinimal, "\"Level\":0", "\"Level\":00"), "expected ',' or '}'");
    syntax("plus sign", mutate(kMinimal, "\"Level\":0", "\"Level\":+1"), "unexpected character");
    syntax("bare fraction", mutate(kMinimal, "\"Level\":0", "\"Level\":.5"), "unexpected character");
    syntax("empty fraction", mutate(kMinimal, "\"Level\":0", "\"Level\":1."), "invalid number fraction");
    syntax("empty exponent", mutate(kMinimal, "\"Level\":0", "\"Level\":1e"), "invalid number exponent");
    syntax("lone minus", mutate(kMinimal, "\"Level\":0", "\"Level\":-"), "invalid number");
    syntax("NaN", mutate(kMinimal, "\"Level\":0", "\"Level\":NaN"), "unexpected character");
    syntax("Infinity", mutate(kMinimal, "\"Level\":0", "\"Level\":Infinity"), "unexpected character");
    syntax("truncated literal", mutate(kMinimal, "true", "tru"), "unexpected character");
    syntax("single quotes", "{'class':false}", "expected a member name");
    syntax("comment", mutate(kMinimal, "{\"class\"", "{/*c*/\"class\""), "expected a member name");
    syntax("missing colon", mutate(kMinimal, "\"class\":false", "\"class\" false"), "expected ':'");
    syntax("unterminated string", "{\"class", "unterminated string");
    syntax("raw control character", mutate(kMinimal, "\"only\"", "\"o\tnly\""), "control character");
    syntax("invalid escape", mutate(kMinimal, "\"only\"", "\"\\q\""), "invalid escape");
    syntax("truncated unicode escape", mutate(kMinimal, "\"only\"", "\"\\u12\""), "invalid \\u escape");
    syntax("non-hex unicode escape", mutate(kMinimal, "\"only\"", "\"\\u12G4\""), "invalid \\u escape");
    syntax("lone high surrogate", mutate(kMinimal, "\"only\"", "\"\\ud800\""), "lone high surrogate");
    syntax("lone low surrogate", mutate(kMinimal, "\"only\"", "\"\\udc00\""), "lone low surrogate");
    syntax("high then non-low", mutate(kMinimal, "\"only\"", "\"\\ud800\\u0041\""), "without a low surrogate");
    syntax("overlong UTF-8", mutate(kMinimal, "only", "\300\200"), "ill-formed UTF-8");
    syntax("encoded surrogate UTF-8", mutate(kMinimal, "only", "\355\240\200"), "ill-formed UTF-8");
    syntax("above U+10FFFF", mutate(kMinimal, "only", "\364\220\200\200"), "ill-formed UTF-8");
    syntax("truncated UTF-8", mutate(kMinimal, "only", "\303"), "ill-formed UTF-8");
    syntax("stray continuation byte", mutate(kMinimal, "only", "\200"), "ill-formed UTF-8");
    syntax("duplicate member", mutate(kMinimal, "{\"class\":false,", "{\"class\":false,\"class\":false,"), "duplicate object member");
    syntax("duplicate after unescaping", mutate(kMinimal, "{\"class\":false,", "{\"class\":false,\"cl\\u0061ss\":false,"), "duplicate object member");
    syntax("duplicate nested", mutate(kMinimal, "\"Child\":{\"Label\":\"\"", "\"Child\":{\"Label\":\"\",\"Label\":\"\""), "duplicate object member");
    syntax("duplicate inside an unknown subtree", mutate(kMinimal, "{\"class\"", "{\"Extra\":{\"a\":1,\"a\":2},\"class\""), "duplicate object member");
    // Resource guard: a deep nesting bomb is an error, not a crash.
    syntax("array nesting bomb", std::string(200000, '['), "nesting is too deep");
    std::string objects;
    for (int i = 0; i < 50000; ++i) {
        objects += "{\"a\":";
    }
    syntax("object nesting bomb", objects, "nesting is too deep");
}

void check_model_rejections() {
    rejects("root array", "[]", "Payload", "expected object, found array");
    rejects("root null", "null", "Payload", "expected object, found null");
    rejects("root string", "\"x\"", "Payload", "expected object, found string");
    rejects("null Boolean", mutate(kMinimal, "\"Enabled\":true", "\"Enabled\":null"), "Payload.Enabled", "expected boolean, found null");
    rejects("string Boolean", mutate(kMinimal, "\"Enabled\":true", "\"Enabled\":\"true\""), "Payload.Enabled", "expected boolean, found string");
    rejects("number Boolean", mutate(kMinimal, "\"Enabled\":true", "\"Enabled\":1"), "Payload.Enabled", "expected boolean, found number");
    rejects("named Boolean wrong kind", mutate(kMinimal, "\"class\":false", "\"class\":0"), "Payload.class", "expected boolean");
    rejects("Boolean as integer", mutate(kMinimal, "\"Level\":0", "\"Level\":false"), "Payload.Level", "expected number, found boolean");
    rejects("integer as string", mutate(kMinimal, "\"Level\":0", "\"Level\":\"1\""), "Payload.Level", "expected number, found string");
    rejects("fractional integer", mutate(kMinimal, "\"Level\":0", "\"Level\":1.0"), "Payload.Level", "expected an integer");
    rejects("exponent integer", mutate(kMinimal, "\"Level\":0", "\"Level\":1e1"), "Payload.Level", "expected an integer");
    rejects("Level above max", mutate(kMinimal, "\"Level\":0", "\"Level\":11"), "Payload.Level", "domain check");
    rejects("Level below min", mutate(kMinimal, "\"Level\":0", "\"Level\":-11"), "Payload.Level", "domain check");
    rejects("short above max", mutate(kMinimal, "\"Small\":0", "\"Small\":32768"), "Payload.Small", "domain check");
    rejects("short below min", mutate(kMinimal, "\"Small\":0", "\"Small\":-32769"), "Payload.Small", "domain check");
    rejects("unsigned byte above max", mutate(kMinimal, "\"Count\":0", "\"Count\":256"), "Payload.Child.Count", "domain check");
    rejects("unsigned byte negative", mutate(kMinimal, "\"Count\":0", "\"Count\":-1"), "Payload.Child.Count", "out of range");
    rejects("unsigned int above max", mutate(kMinimal, "\"Wide\":0", "\"Wide\":4294967296"), "Payload.Wide", "domain check");
    rejects("int64 above max", mutate(kMinimal, "\"Offset\":0", "\"Offset\":9223372036854775808"), "Payload.Offset", "out of range");
    rejects("int64 below min", mutate(kMinimal, "\"Offset\":0", "\"Offset\":-9223372036854775809"), "Payload.Offset", "out of range");
    rejects("integer of 40 digits", mutate(kMinimal, "\"Offset\":0", "\"Offset\":1234567890123456789012345678901234567890"), "Payload.Offset", "out of range");
    rejects("integer of 5000 digits", mutate(kMinimal, "\"Offset\":0", "\"Offset\":" + std::string(5000, '9')), "Payload.Offset", "out of range");
    rejects("unknown enumerator (case)", mutate(kMinimal, "\"Signal\":\"Self\"", "\"Signal\":\"self\""), "Payload.Signal", "not a member of enumeration SignalCode");
    rejects("enumerator by C++ name", mutate(kMinimal, "\"Signal\":\"Self\"", "\"Signal\":\"Value5G\""), "Payload.Signal", "not a member of enumeration SignalCode");
    rejects("empty enumerator", mutate(kMinimal, "\"Signal\":\"Self\"", "\"Signal\":\"\""), "Payload.Signal", "not a member of enumeration SignalCode");
    rejects("numeric enumerator", mutate(kMinimal, "\"Signal\":\"Self\"", "\"Signal\":1"), "Payload.Signal", "expected string, found number");
    rejects("number String", mutate(kMinimal, "{\"class\"", "{\"Note\":5,\"class\""), "Payload.Note", "expected string, found number");
    rejects("number in String list", mutate(kMinimal, "[\"only\"]", "[\"only\",7]"), "Payload.Names[1]", "expected string, found number");
    rejects("unknown member", mutate(kMinimal, "{\"class\"", "{\"Extra\":1,\"class\""), "Payload", "unknown member \"Extra\"");
    rejects("member name is case sensitive", mutate(kMinimal, "\"class\"", "\"Class\""), "Payload", "unknown member \"Class\"");
    rejects("host member name is not the wire name", mutate(kMinimal, "\"class\"", "\"field_class\""), "Payload", "unknown member \"field_class\"");
    rejects("unknown nested member", mutate(kMinimal, "\"Child\":{", "\"Child\":{\"Extra\":1,"), "Payload.Child", "unknown member \"Extra\"");
    rejects("missing required member", mutate(kMinimal, "\"Enabled\":true,", ""), "Payload", "missing required member \"Enabled\"");
    rejects("missing nested required member", mutate(kMinimal, "\"Key\":0", "\"Name\":\"n\""), "Payload.Identity", "missing required member \"Key\"");
    rejects("$type names another type", mutate(kMinimal, "{\"class\"", "{\"$type\":\"Other\",\"class\""), "Payload", "\"$type\"");
    rejects("$type is namespace-qualified wrongly", mutate(kMinimal, "{\"class\"", "{\"$type\":\"{urn:x}Payload\",\"class\""), "Payload", "\"$type\"");
    rejects("$type is not a string", mutate(kMinimal, "{\"class\"", "{\"$type\":5,\"class\""), "Payload", "\"$type\"");
    rejects("nested $type names a derived type", mutate(kMinimal, "\"Identity\":{", "\"Identity\":{\"$type\":\"IdentityBase\","), "Payload.Identity", "\"$type\"");
    rejects("too few occurrences", mutate(kMinimal, "[0,0]", "[0]"), "Payload.Levels", "occurrence(s) rejected");
    rejects("too many occurrences", mutate(kMinimal, "[0,0]", "[0,0,0,0]"), "Payload.Levels", "occurrence(s) rejected");
    rejects("too many occurrences (enumeration)", mutate(kMinimal, "\"Levels\"", "\"Signals\":[\"Self\",\"Self\",\"Self\",\"Self\",\"Self\"],\"Levels\""), "Payload.Signals", "5 occurrence(s) rejected");
    rejects("empty required repetition", mutate(kMinimal, "[\"only\"]", "[]"), "Payload.Names", "0 occurrence(s) rejected");
    rejects("absent required repetition", mutate(kMinimal, ",\"Names\":[\"only\"]", ""), "Payload.Names", "0 occurrence(s) rejected");
    rejects("repetition is an object", mutate(kMinimal, "\"Levels\"", "\"Flags\":{},\"Levels\""), "Payload.Flags", "expected array, found object");
    rejects("repetition is a scalar", mutate(kMinimal, "\"Levels\"", "\"Flags\":true,\"Levels\""), "Payload.Flags", "expected array, found boolean");
    rejects("null array element", mutate(kMinimal, "\"Levels\"", "\"Flags\":[true,null],\"Levels\""), "Payload.Flags[1]", "expected boolean, found null");
    rejects("bad repeated Record element", mutate(kMinimal, "\"Levels\"", "\"Children\":[{\"Label\":\"a\",\"Count\":0},{\"Label\":\"b\",\"Count\":300}],\"Levels\""), "Payload.Children[1].Count", "domain check");
    rejects("too many repeated Records", mutate(kMinimal, "\"Levels\"", "\"Children\":[{\"Label\":\"a\",\"Count\":0},{\"Label\":\"a\",\"Count\":0},{\"Label\":\"a\",\"Count\":0}],\"Levels\""), "Payload.Children", "3 occurrence(s) rejected");
    rejects("bad unbounded element", mutate(kMinimal, "\"Levels\"", "\"Ids\":[1,-2],\"Levels\""), "Payload.Ids[1]", "out of range");
    rejects("optional present as null", mutate(kMinimal, "\"Levels\"", "\"Note\":null,\"Levels\""), "Payload.Note", "expected string, found null");
    rejects("optional Record present as array", mutate(kMinimal, "\"Levels\"", "\"Empty\":[],\"Levels\""), "Payload.Empty", "expected object, found array");
    rejects("empty Record with a member", mutate(kMinimal, "\"Levels\"", "\"Empty\":{\"x\":1},\"Levels\""), "Payload.Empty", "unknown member \"x\"");
    {
        const auto ack = sc::decode_Ack(R"({"Accepted":true,"Accepted":false})");
        CHECK(!ack.ok() && ack.error().path == "$");
        const auto wrong = sc::decode_Ack(R"({"Accepted":"yes"})");
        CHECK(!wrong.ok() && wrong.error().path == "Ack.Accepted");
    }
}

void check_encode_rejections() {
    {
        m::Payload bad = make_minimal();
        bad.signal = static_cast<m::SignalCode>(99);
        const auto result = sc::encode_Payload(bad);
        CHECK(!result.ok());
        CHECK(result.error().path == "Payload.Signal");
        CHECK(result.error().message.find("not an enumerator of SignalCode") != std::string::npos);
    }
    {
        m::Payload bad = make_minimal();
        bad.note = std::string("bad\377");
        const auto result = sc::encode_Payload(bad);
        CHECK(!result.ok() && result.error().path == "Payload.Note");
        CHECK(result.error().message.find("UTF-8") != std::string::npos);
    }
    {
        m::Payload bad = make_minimal();
        bad.child.label = "\355\240\200";  // an encoded surrogate
        const auto result = sc::encode_Payload(bad);
        CHECK(!result.ok() && result.error().path == "Payload.Child.Label");
    }
    {
        m::Payload bad = make_minimal();
        bad.names = must<m::UnboundedVector<std::string, 1>>(std::vector<std::string>{"ok", "\300\257"});
        const auto result = sc::encode_Payload(bad);
        CHECK(!result.ok() && result.error().path == "Payload.Names[1]");
    }
    {
        m::Payload bad = make_minimal();
        bad.signals = must<m::BoundedVector<m::SignalCode, 0, 4>>(
            std::vector<m::SignalCode>{m::SignalCode::Self, static_cast<m::SignalCode>(-1)});
        const auto result = sc::encode_Payload(bad);
        CHECK(!result.ok() && result.error().path == "Payload.Signals[1]");
    }
    // Every ASCII control character is escaped; DEL and non-ASCII pass through.
    m::Payload controls = make_minimal();
    std::string all;
    for (int c = 1; c < 0x20; ++c) {
        all += static_cast<char>(c);
    }
    all += "\177";
    controls.note = all;
    const auto encoded = sc::encode_Payload(controls);
    CHECK(encoded.ok());
    if (encoded.ok()) {
        CHECK(encoded.value().find("\"Note\":\"\\u0001\\u0002") != std::string::npos);
        CHECK(encoded.value().find("\\b\\t\\n\\u000b\\f\\r") != std::string::npos);
        const auto back = sc::decode_Payload(encoded.value());
        CHECK(back.ok() && back.value().note && *back.value().note == all);
    }
}

/// White-box: the integer readers cover the full 64-bit ranges even though the
/// XSD frontend's widest unsigned built-in is 32 bits wide.
void check_integer_boundaries_in_runtime() {
    namespace d = sc::detail;
    const auto read_u64 = [](const std::string& token) {
        sc::CodecError error;
        d::Value tree;
        if (!d::parse_json(token, tree, error)) {
            return std::optional<std::uint64_t>();
        }
        return d::get_u64(tree, error, nullptr);
    };
    const auto read_i64 = [](const std::string& token) {
        sc::CodecError error;
        d::Value tree;
        if (!d::parse_json(token, tree, error)) {
            return std::optional<std::int64_t>();
        }
        return d::get_i64(tree, error, nullptr);
    };
    CHECK(read_u64("0") == std::optional<std::uint64_t>(0));
    CHECK(read_u64("-0") == std::optional<std::uint64_t>(0));
    CHECK(read_u64("18446744073709551615") == std::optional<std::uint64_t>(UINT64_MAX));
    CHECK(!read_u64("18446744073709551616"));
    CHECK(!read_u64("99999999999999999999"));
    CHECK(!read_u64("-1"));
    CHECK(!read_u64("1.5"));
    CHECK(!read_u64("\"1\""));
    CHECK(read_i64("9223372036854775807") == std::optional<std::int64_t>(INT64_MAX));
    CHECK(read_i64("-9223372036854775808") == std::optional<std::int64_t>(INT64_MIN));
    CHECK(!read_i64("9223372036854775808"));
    CHECK(!read_i64("-9223372036854775809"));
    CHECK(!read_i64("18446744073709551615"));
    std::string out;
    CHECK(d::put_u64(out, UINT64_MAX) && out == "18446744073709551615");
}

/// Robustness without a sanitizer oracle: no proper prefix of a valid document
/// is accepted, and single-byte corruption never crashes the decoder.
void check_fuzz_style_robustness() {
    for (std::size_t length = 0; length < kCanonical.size(); ++length) {
        CHECK(!sc::decode_Payload(std::string_view(kCanonical).substr(0, length)).ok());
    }
    const char replacements[] = {'\0', '"', '\\', '{', '}', '[', ']', ',', ':', '-', '0', 'e', '\377', 'u'};
    for (std::size_t at = 0; at < kCanonical.size(); ++at) {
        for (const char replacement : replacements) {
            const auto result = sc::decode_Payload(flipped(kCanonical, at, replacement));
            // Either outcome is legitimate (e.g. digit-for-digit); it must
            // simply not crash, and a success must re-encode successfully.
            if (result.ok()) {
                CHECK(sc::encode_Payload(result.value()).ok());
            } else {
                CHECK(!result.error().message.empty());
            }
        }
    }
}

}  // namespace

int main() {
    check_round_trip_and_exact_wire();
    check_non_canonical_input_is_accepted();
    check_syntax_rejections();
    check_model_rejections();
    check_encode_rejections();
    check_integer_boundaries_in_runtime();
    check_fuzz_style_robustness();
    std::cout << checks << " checks, " << failures << " failures\n";
    return failures == 0 ? 0 : 1;
}
