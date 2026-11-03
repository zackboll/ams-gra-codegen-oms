// CPP-OMS-006 consumer: compiled against the GENERATED model and service_codec
// headers for the shared Task 059 synthetic fixture
// tests/fixtures/service-generate/codec-structured-ascii.{xsd,yaml}
// (StructuredNotice / StructuredPayload: IMO number, overlapping-prefix PRF
// code, file name, octal value and model code, in required, optional, bounded,
// unbounded and Choice positions).
//
//   structured_ascii_consumer         -> CPP_OMS_006_STRUCTURED_ASCII checks=N rejections=M
//
// Exit status 0 and the summary line mean every check passed. Built by the
// registered cpp_codec.rs tests, PR smoke and
// scripts/check-cpp-oms-006-file-metadata.sh with strict C++17 flags and
// sanitizers.
//
// Independence: every JSON document and every expected encoding below is
// written by hand (compact, schema order). None comes from the encoder under
// test. The expected verdicts follow the XSD patterns and facets, not the
// generated code. The codec may only call the model's `create`, so a mutation
// in the model's validator or in the codec's use of it changes a verdict here
// (see tests/cpp-codec/structured_ascii_mutations.py).
#include <cstddef>
#include <cstdlib>
#include <initializer_list>
#include <iostream>
#include <map>
#include <optional>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

#include "oam.hpp"
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

// ---- carrier construction policy (the encode-side invariant) ----------------
//
// A structured-ASCII value can only come from `create`, so the encoder (which
// writes `value()` unchanged) can never see a value outside the profile.
template <typename T>
constexpr bool only_create =
    !std::is_default_constructible<T>::value && !std::is_constructible<T, std::string>::value &&
    !std::is_constructible<T, std::string_view>::value && !std::is_constructible<T, const char*>::value &&
    std::is_copy_constructible<T>::value &&
    std::is_same<decltype(T::create(std::string_view())), std::optional<T>>::value &&
    std::is_same<decltype(std::declval<const T&>().value()), const std::string&>::value;
static_assert(only_create<m::ImoType>, "ImoType construction policy");
static_assert(only_create<m::PrfType>, "PrfType construction policy");
static_assert(only_create<m::FileNameType>, "FileNameType construction policy");
static_assert(only_create<m::OctalType>, "OctalType construction policy");
static_assert(only_create<m::ModelType>, "ModelType construction policy");
static_assert(std::is_same<decltype(m::StructuredPayload::filename), std::optional<m::FileNameType>>::value,
              "FileName is std::optional<FileNameType>");

/// A generated factory that must accept its argument (test setup only).
template <typename T>
T must(std::string_view text, int line = __builtin_LINE()) {
    std::optional<T> created = T::create(text);
    if (!created) {
        std::cerr << "test setup (line " << line << "): a generated factory rejected its argument\n";
        std::exit(2);
    }
    return *created;
}

// ---- hand-authored documents -----------------------------------------------

const std::vector<std::string> kOrder = {"Imo", "Prf", "FileName", "Imos", "Octals", "Pick"};

const std::map<std::string, std::string> kBase = {
    {"Imo", R"("IMO1234567")"},
    {"Prf", R"("178")"},
    {"Pick", R"({"Model":"aZ9"})"},
};

using Change = std::pair<std::string, std::optional<std::string>>;

/// The base document with members replaced (`nullopt` removes one), written in
/// schema order; keys outside the schema are appended at the end.
std::string doc(std::initializer_list<Change> changes = {}) {
    std::map<std::string, std::string> members = kBase;
    std::vector<std::string> extra;
    for (const Change& change : changes) {
        bool known = false;
        for (const std::string& key : kOrder) {
            known = known || key == change.first;
        }
        if (!known) {
            extra.push_back("\"" + change.first + "\":" + *change.second);
            continue;
        }
        if (change.second) {
            members[change.first] = *change.second;
        } else {
            members.erase(change.first);
        }
    }
    std::string out = "{";
    bool first = true;
    for (const std::string& key : kOrder) {
        const auto found = members.find(key);
        if (found == members.end()) {
            continue;
        }
        out += first ? "" : ",";
        first = false;
        out += "\"" + key + "\":" + found->second;
    }
    for (const std::string& member : extra) {
        out += first ? "" : ",";
        first = false;
        out += member;
    }
    return out + "}";
}

/// Quote already-JSON-escaped text (the samples below spell their escapes).
std::string quoted(const std::string& text) { return "\"" + text + "\""; }

// ---- assertions --------------------------------------------------------------

using Payload = m::StructuredPayload;

/// Decode must succeed; the typed value is checked by `inspect`; encoding the
/// decoded value must give exactly `expected` (the input unless stated).
template <typename Inspect>
void accept(const std::string& json, Inspect inspect, const std::string& expected) {
    const auto decoded = sc::decode_StructuredPayload(json);
    ++checks;
    if (!decoded.ok()) {
        ++failures;
        std::cerr << "rejected valid document " << json << ": " << decoded.error().path << ": "
                  << decoded.error().message << '\n';
        return;
    }
    inspect(decoded.value());
    const auto encoded = sc::encode_StructuredPayload(decoded.value());
    ++checks;
    if (!encoded.ok() || encoded.value() != expected) {
        ++failures;
        std::cerr << "re-encoding mismatch for " << json << "\n  expected " << expected << "\n  actual   "
                  << (encoded.ok() ? encoded.value() : encoded.error().path + ": " + encoded.error().message)
                  << '\n';
    }
}

void accept(const std::string& json) {
    accept(json, [](const Payload&) {}, json);
}

template <typename Inspect>
void accept(const std::string& json, Inspect inspect) {
    accept(json, inspect, json);
}

/// Decode must fail at exactly `path` with exactly `message`.
void reject(const std::string& json, const std::string& path, const std::string& message) {
    const auto decoded = sc::decode_StructuredPayload(json);
    ++checks;
    ++rejections;
    if (decoded.ok()) {
        ++failures;
        std::cerr << "accepted invalid document " << json << " (expected " << path << ")\n";
        return;
    }
    if (decoded.error().path != path || decoded.error().message != message) {
        ++failures;
        std::cerr << "wrong rejection for " << json << "\n  expected " << path << ": " << message
                  << "\n  actual   " << decoded.error().path << ": " << decoded.error().message << '\n';
    }
}

/// Decode must fail at `$` with a message that STARTS with `prefix` (JSON
/// syntax diagnostics carry a byte offset after the stable prefix).
void reject_json(const std::string& json, const std::string& prefix) {
    const auto decoded = sc::decode_StructuredPayload(json);
    ++checks;
    ++rejections;
    if (decoded.ok()) {
        ++failures;
        std::cerr << "accepted malformed document " << json << '\n';
        return;
    }
    if (decoded.error().path != "$" || decoded.error().message.rfind(prefix, 0) != 0) {
        ++failures;
        std::cerr << "wrong JSON rejection for " << json << "\n  expected $: " << prefix << "..."
                  << "\n  actual   " << decoded.error().path << ": " << decoded.error().message << '\n';
    }
}

const std::string kRoot = "StructuredPayload";

std::string model_check(const std::string& type) {
    return "value rejected by the generated model's " + type + " check";
}

/// Reject `raw` (a JSON value) at member `key` of the base document.
void reject_member(const std::string& key, const std::string& raw, const std::string& type) {
    reject(doc({{key, raw}}), kRoot + "." + key, model_check(type));
}

void reject_kind(const std::string& key, const std::string& raw, const std::string& found) {
    reject(doc({{key, raw}}), kRoot + "." + key, "expected string, found " + found);
}

const std::vector<std::pair<std::string, std::string>> kWrongKinds = {
    {"0", "number"}, {"1234567", "number"}, {"null", "null"}, {"[]", "array"},
    {R"(["IMO1234567"])", "array"}, {"{}", "object"}, {"true", "boolean"}};

// ---- suites ------------------------------------------------------------------

/// FileNameType [a-zA-Z0-9_\-]+\.[a-zA-Z0-9_\.\-]+, minLength 1, maxLength 255,
/// as the OPTIONAL member FileName.
void file_name() {
    accept(doc(), [](const Payload& v) { CHECK(!v.filename.has_value()); });
    // Ordinary names and multiple extensions; the first class excludes '.', so
    // the first dot is the boundary and later dots belong to the extension.
    for (const char* good : {"report.txt", "report.tar.bz2", "a.b", "A.B", "_.-", "-x.y_z", "file-01_v2.log",
                             "archive.tar.gz.part1", "x.y.", "x..", "Z.9", "0.0", "a_b-c.D-E_F.1"}) {
        accept(doc({{"FileName", quoted(good)}}),
               [good](const Payload& v) { CHECK(v.filename && v.filename->value() == good); });
    }
    // Missing or empty segments.
    for (const char* bad : {"report.", ".txt", ".", "..", "report", "", ".tar.bz2", "a"}) {
        reject_member("FileName", quoted(bad), "FileNameType");
    }
    // Character-class violations in each segment and around the boundary.
    for (const char* bad : {"re port.txt", "report.t xt", "report .txt", " report.txt", "report.txt ",
                            "a/b.txt", "a.b/c", "a\\\\b.txt", "a:b.txt", "a.b:c", "a+b.txt", "a.b~",
                            "a*.txt", "a.b,c", "a@b.c", "a#.b", "a.b!"}) {
        reject_member("FileName", quoted(bad), "FileNameType");
    }
    // whiteSpace = preserve: TAB/LF/CR and NBSP are not collapsed or trimmed.
    for (const char* bad : {"report.txt\\t", "\\treport.txt", "report.\\ttxt", "report\\n.txt", "report.txt\\r",
                            "report.txt\\u00A0", "report\\u0000.txt", "report.txt\\u007F"}) {
        reject_member("FileName", quoted(bad), "FileNameType");
    }
    // Non-ASCII UTF-8 (2-, 3- and 4-byte) and escaped non-ASCII.
    for (const char* bad : {"résumé.txt", "report.tx\xC3\xA9", "\xE2\x82\xAC.txt", "a.\xF0\x9F\x98\x80",
                            "\\u00e9.txt", "a.\\u20AC", "ｒeport.txt"}) {
        reject_member("FileName", quoted(bad), "FileNameType");
    }
    // Escapes of class members decode to exactly those characters.
    accept(doc({{"FileName", R"("\u0061.\u0062")"}}),
           [](const Payload& v) { CHECK(v.filename && v.filename->value() == "a.b"); }, doc({{"FileName", R"("a.b")"}}));
    accept(doc({{"FileName", R"("a\u002eb")"}}),
           [](const Payload& v) { CHECK(v.filename && v.filename->value() == "a.b"); }, doc({{"FileName", R"("a.b")"}}));
    reject_member("FileName", R"("a\/b.c")", "FileNameType");
    // Length boundaries: 255 accepted, 256 rejected (the pattern alone has no
    // upper bound, so this is the maxLength facet).
    const std::string max255 = std::string(127, 'n') + "." + std::string(127, 'e');
    const std::string over256 = std::string(128, 'n') + "." + std::string(127, 'e');
    CHECK(max255.size() == 255 && over256.size() == 256);
    accept(doc({{"FileName", quoted(max255)}}),
           [&max255](const Payload& v) { CHECK(v.filename && v.filename->value() == max255); });
    accept(doc({{"FileName", quoted("a." + std::string(253, '.'))}}));
    reject_member("FileName", quoted(over256), "FileNameType");
    reject_member("FileName", quoted("a." + std::string(254, 'x')), "FileNameType");
    for (const auto& kind : kWrongKinds) {
        reject_kind("FileName", kind.first, kind.second);
    }
}

/// ImoType IMO[0-9]{7}, length 10, as the REQUIRED member Imo and the bounded
/// 0..3 member Imos.
void imo_number() {
    for (const char* good : {"IMO0000000", "IMO9999999", "IMO1234567"}) {
        accept(doc({{"Imo", quoted(good)}}), [good](const Payload& v) { CHECK(v.imo.value() == good); });
    }
    for (const char* bad : {"imo1234567", "Imo1234567", "IMo1234567", "IM01234567", "IMX1234567", "1234567IMO",
                            " IMO123456", "IMO 123456", "IMO123456", "IMO12345678", "IMO123456A", "IMO12345.7",
                            "IMO", "", "IMOIMO1234", "IMO\\t123456", "IMO１２３４５６７", "IMO123456\\u0000"}) {
        reject_member("Imo", quoted(bad), "ImoType");
    }
    for (const auto& kind : kWrongKinds) {
        reject_kind("Imo", kind.first, kind.second);
    }
    reject(doc({{"Imo", std::nullopt}}), kRoot, "missing required member \"Imo\"");

    // Bounded repetition 0..3.
    accept(doc({{"Imos", R"(["IMO0000001","IMO0000002","IMO0000003"])"}}), [](const Payload& v) {
        CHECK(v.imos.values().size() == 3);
        CHECK(v.imos.values().size() == 3 && v.imos.values()[2].value() == "IMO0000003");
    });
    accept(doc({{"Imos", R"(["IMO7654321"])"}}));
    accept(doc({{"Imos", "[]"}}), [](const Payload& v) { CHECK(v.imos.values().empty()); }, doc());
    reject(doc({{"Imos", R"(["IMO0000001","imo0000002"])"}}), kRoot + ".Imos[1]", model_check("ImoType"));
    reject(doc({{"Imos", R"(["IMO0000001","IMO0000002","IMO000003"])"}}), kRoot + ".Imos[2]",
           model_check("ImoType"));
    reject(doc({{"Imos", R"(["IMO0000001",1234567])"}}), kRoot + ".Imos[1]", "expected string, found number");
    reject(doc({{"Imos", R"(["IMO0000001",null])"}}), kRoot + ".Imos[1]", "expected string, found null");
    reject(doc({{"Imos", R"(["IMO0000001","IMO0000002","IMO0000003","IMO0000004"])"}}), kRoot + ".Imos",
           "4 occurrence(s) rejected by the generated sequence constructor");
    reject(doc({{"Imos", R"("IMO0000001")"}}), kRoot + ".Imos", "expected array, found string");
    reject(doc({{"Imos", "null"}}), kRoot + ".Imos", "expected array, found null");
}

/// PrfType 1?[1-7][1-8]{2}, minLength 3, maxLength 4: the optional literal 1
/// OVERLAPS the following [1-7]; total length selects the partition.
void prf_code() {
    // Length 3: no prefix. Length 4: the prefix must be the literal 1.
    for (const char* good : {"111", "178", "188", "711", "788", "1111", "1178", "1188", "1788", "1711"}) {
        accept(doc({{"Prf", quoted(good)}}), [good](const Payload& v) { CHECK(v.prf.value() == good); });
    }
    // Numeric class restrictions: [1-7] excludes 0, 8, 9; [1-8] excludes 0, 9.
    for (const char* bad : {"078", "878", "978", "170", "179", "107", "189", "1078", "1878", "1179", "1709"}) {
        reject_member("Prf", quoted(bad), "PrfType");
    }
    // Optional prefix: at length 4 the first character must be 1 (no other
    // partition is tried); 2778 is not "2" + "778".
    for (const char* bad : {"2778", "7111", "0178", "8178", "A178"}) {
        reject_member("Prf", quoted(bad), "PrfType");
    }
    // Length facets: 2 and 5 characters; a doubled prefix.
    for (const char* bad : {"17", "1", "", "11178", "11111", "11788"}) {
        reject_member("Prf", quoted(bad), "PrfType");
    }
    // Non-digits, spaces and non-ASCII digits.
    for (const char* bad : {"17 8", " 178", "178 ", "1-78", "١٧٨", "17\\t8", "+178"}) {
        reject_member("Prf", quoted(bad), "PrfType");
    }
    for (const auto& kind : kWrongKinds) {
        reject_kind("Prf", kind.first, kind.second);
    }
    reject_kind("Prf", "178", "number");
    reject_kind("Prf", "1.78e2", "number");
    reject(doc({{"Prf", std::nullopt}}), kRoot, "missing required member \"Prf\"");
}

/// OctalType [0-7]+, minLength 1, maxLength 16, as the UNBOUNDED member Octals.
void octal_values() {
    accept(doc({{"Octals", R"(["0"])"}}));
    accept(doc({{"Octals", R"(["7777777777777777","01234567","0"])"}}), [](const Payload& v) {
        CHECK(v.octals.values().size() == 3);
        CHECK(v.octals.values().size() == 3 && v.octals.values()[0].value().size() == 16);
    });
    std::string many = "[";
    for (int i = 0; i < 40; ++i) {
        many += (i == 0 ? "" : ",") + std::string("\"") + std::to_string(i % 8) + "\"";
    }
    many += "]";
    accept(doc({{"Octals", many}}), [](const Payload& v) { CHECK(v.octals.values().size() == 40); });
    for (const char* bad : {"8", "9", "08", "", "77777777777777777", "0x7", "-1", "7 ", "0o7"}) {
        reject(doc({{"Octals", "[\"0\",\"" + std::string(bad) + "\"]"}}), kRoot + ".Octals[1]",
               model_check("OctalType"));
    }
    reject(doc({{"Octals", R"(["0",7])"}}), kRoot + ".Octals[1]", "expected string, found number");
    reject(doc({{"Octals", "7"}}), kRoot + ".Octals", "expected array, found number");
}

/// Choice alternatives: ModelType [A-Za-z0-9]{1,4} with maxLength 4 only (no
/// minLength facet: the pattern alone forbids ""), and ImoType.
void choice() {
    for (const char* good : {"a", "Z", "09", "aZ9", "aZ09", "ZZZZ"}) {
        accept(doc({{"Pick", "{\"Model\":\"" + std::string(good) + "\"}"}}), [good](const Payload& v) {
            const auto* model = std::get_if<m::StructuredPick::Model>(&v.pick.value);
            CHECK(model != nullptr && model->value.value() == good);
        });
    }
    for (const char* bad : {"", "aZ09x", "a-b", "a b", "a_b", "é", "a.b"}) {
        reject(doc({{"Pick", "{\"Model\":\"" + std::string(bad) + "\"}"}}), kRoot + ".Pick.Model",
               model_check("ModelType"));
    }
    accept(doc({{"Pick", R"({"Imo":"IMO7654321"})"}}), [](const Payload& v) {
        const auto* imo = std::get_if<m::StructuredPick::Imo>(&v.pick.value);
        CHECK(imo != nullptr && imo->value.value() == "IMO7654321");
    });
    reject(doc({{"Pick", R"({"Imo":"IMO765432"})"}}), kRoot + ".Pick.Imo", model_check("ImoType"));
    // A valid ModelType text is not a valid ImoType: the alternative decides.
    reject(doc({{"Pick", R"({"Imo":"aZ09"})"}}), kRoot + ".Pick.Imo", model_check("ImoType"));
    reject(doc({{"Pick", R"({"Model":"IMO7654321"})"}}), kRoot + ".Pick.Model", model_check("ModelType"));
    reject(doc({{"Pick", R"({"Model":12})"}}), kRoot + ".Pick.Model", "expected string, found number");
    reject(doc({{"Pick", R"({"Imo":null})"}}), kRoot + ".Pick.Imo", "expected string, found null");
    reject(doc({{"Pick", "{}"}}), kRoot + ".Pick", "a Choice requires exactly one selected member");
    reject(doc({{"Pick", R"({"Model":"a","Imo":"IMO7654321"})"}}), kRoot + ".Pick",
           "a Choice requires exactly one selected member");
    reject(doc({{"Pick", R"({"model":"a"})"}}), kRoot + ".Pick", "unknown member \"model\"");
    reject(doc({{"Pick", R"("aZ09")"}}), kRoot + ".Pick", "expected object, found string");
    reject(doc({{"Pick", "null"}}), kRoot + ".Pick", "expected object, found null");
    reject(doc({{"Pick", std::nullopt}}), kRoot, "missing required member \"Pick\"");
}

/// Document-level JSON rules around structured-ASCII members.
void document_rules() {
    // Duplicate members, also after escape decoding of the key; the second
    // value is valid, so only the duplicate rule can reject.
    reject_json(R"({"Imo":"IMO1234567","Imo":"IMO1234567","Prf":"178","Pick":{"Model":"aZ9"}})",
                "duplicate object member");
    reject_json(R"({"Imo":"IMO1234567","Prf":"178","FileName":"a.b","File\u004eame":"a.b","Pick":{"Model":"aZ9"}})",
                "duplicate object member");
    reject_json(R"({"Imo":"IMO1234567","Prf":"178","Pick":{"Model":"a","Model":"b"}})", "duplicate object member");
    // Unknown members, a case variant and the global-element envelope.
    reject(doc({{"Extra", R"("a.b")"}}), kRoot, "unknown member \"Extra\"");
    reject(doc({{"Filename", R"("a.b")"}}), kRoot, "unknown member \"Filename\"");
    // "$type" is accepted only when it names the concrete type, and is never
    // written back.
    accept(doc({{"$type", R"("StructuredPayload")"}}), [](const Payload&) {}, doc());
    reject(doc({{"$type", R"("StructuredPick")"}}), kRoot, "\"$type\" does not name the concrete type being decoded");
    reject("{\"StructuredNotice\":" + doc() + "}", kRoot, "unknown member \"StructuredNotice\"");
    // Raw control characters and ill-formed UTF-8 are JSON errors at "$".
    reject_json(doc({{"FileName", "\"report\t.txt\""}}), "unescaped control character in string");
    reject_json(doc({{"FileName", "\"a.b\x01\""}}), "unescaped control character in string");
    reject_json(doc({{"FileName", "\"a.\xC3\x28\""}}), "ill-formed UTF-8 in string");
    reject_json(doc({{"FileName", "\"a.\xC0\xAE\""}}), "ill-formed UTF-8 in string");
    reject_json(doc({{"FileName", R"("a.\udc00")"}}), "lone low surrogate escape");
    reject_json(doc() + " x", "unexpected content after the JSON value");
    reject(R"([{"Imo":"IMO1234567","Prf":"178","Pick":{"Model":"aZ9"}}])", kRoot, "expected object, found array");
    reject("null", kRoot, "expected object, found null");
    // Insignificant whitespace and member order do not change the value; the
    // encoder writes schema order with no whitespace.
    accept(" { \"Pick\" : { \"Model\" : \"aZ9\" } , \"FileName\" : \"a.tar.gz\" , \"Prf\" : \"1178\" ,\n"
           "\t\"Imo\" : \"IMO1234567\" } ",
           [](const Payload& v) {
               CHECK(v.filename && v.filename->value() == "a.tar.gz");
               CHECK(v.prf.value() == "1178");
           },
           R"({"Imo":"IMO1234567","Prf":"1178","FileName":"a.tar.gz","Pick":{"Model":"aZ9"}})");
}

/// Encoding a value built through the model API (not through decode) gives the
/// hand-written canonical text: schema order, absent optionals and empty
/// repetitions omitted, values written verbatim.
void encode_from_model() {
    Payload value{
        must<m::ImoType>("IMO0012345"),
        must<m::PrfType>("1788"),
        must<m::FileNameType>("report.tar.bz2"),
        *m::BoundedVector<m::ImoType, 0, 3>::create({must<m::ImoType>("IMO1111111"), must<m::ImoType>("IMO2222222")}),
        *m::UnboundedVector<m::OctalType, 0>::create({must<m::OctalType>("17"), must<m::OctalType>("0")}),
        m::StructuredPick{m::StructuredPick::Imo{must<m::ImoType>("IMO3333333")}},
    };
    const std::string expected =
        R"({"Imo":"IMO0012345","Prf":"1788","FileName":"report.tar.bz2","Imos":["IMO1111111","IMO2222222"],)"
        R"("Octals":["17","0"],"Pick":{"Imo":"IMO3333333"}})";
    const auto encoded = sc::encode_StructuredPayload(value);
    CHECK(encoded.ok());
    CHECK(encoded.ok() && encoded.value() == expected);
    if (encoded.ok() && encoded.value() != expected) {
        std::cerr << "  expected " << expected << "\n  actual   " << encoded.value() << '\n';
    }
    const auto decoded = sc::decode_StructuredPayload(expected);
    CHECK(decoded.ok());
    if (decoded.ok()) {
        CHECK(decoded.value().filename && decoded.value().filename->value() == "report.tar.bz2");
        CHECK(decoded.value().imos.values().size() == 2);
        CHECK(decoded.value().octals.values().size() == 2);
    }

    value.filename.reset();
    value.imos = *m::BoundedVector<m::ImoType, 0, 3>::create({});
    value.octals = *m::UnboundedVector<m::OctalType, 0>::create({});
    value.pick = m::StructuredPick{m::StructuredPick::Model{must<m::ModelType>("Q")}};
    const auto minimal = sc::encode_StructuredPayload(value);
    CHECK(minimal.ok() && minimal.value() == R"({"Imo":"IMO0012345","Prf":"1788","Pick":{"Model":"Q"}})");
}

/// The model's own `create` is exercised directly; the codec must delegate to
/// it, so these verdicts must agree with the decode verdicts above.
void model_factories() {
    CHECK(m::FileNameType::create("report.tar.bz2").has_value());
    CHECK(!m::FileNameType::create("report.").has_value());
    CHECK(!m::FileNameType::create(std::string_view("a.b\0", 4)).has_value());
    CHECK(m::PrfType::create("178").has_value());
    CHECK(m::PrfType::create("1178").has_value());
    CHECK(!m::PrfType::create("2778").has_value());
    CHECK(!m::PrfType::create("189").has_value());
    CHECK(m::ImoType::create("IMO1234567").has_value());
    CHECK(!m::ImoType::create("IMO123456").has_value());
    CHECK(!m::ModelType::create("").has_value());
    CHECK(m::OctalType::create("7777777777777777").has_value());
    CHECK(!m::OctalType::create("77777777777777777").has_value());
}

}  // namespace

int main() {
    file_name();
    imo_number();
    prf_code();
    octal_values();
    choice();
    document_rules();
    encode_from_model();
    model_factories();
    if (failures != 0) {
        std::cerr << failures << " failure(s) of " << checks << " checks\n";
        return 1;
    }
    std::cout << "CPP_OMS_006_STRUCTURED_ASCII checks=" << checks << " rejections=" << rejections << '\n';
    return 0;
}
