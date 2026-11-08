// CPP-OMS-005 consumer: compiled against the GENERATED model and service_codec
// headers for the synthetic fixture tests/fixtures/cpp-codec/bounded-ascii
// (every constrained String is a pinned Task 058 bounded-ASCII row).
//
//   bounded_ascii_consumer            -> CPP_OMS_005_BOUNDED_ASCII checks=N rejections=M
//
// Exit status 0 and the summary line mean every check passed. Built by the
// registered cpp_codec.rs tests, PR smoke and
// scripts/check-cpp-oms-005-amti-settings.sh with strict C++17 flags and
// sanitizers.
//
// Independence: every JSON document and every expected encoding below is
// written by hand here (compact, schema order). None is produced by the encoder
// under test. The expected verdicts follow the XSD facets, not the generated
// code: the codec must only call the model's `create`, so a mutation in either
// the model's validator or the codec's use of it changes a verdict here.
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
// A bounded-ASCII value can only come from `create`, so the encoder (which
// writes `value()` unchanged) can never see a value outside the profile.
template <typename T>
constexpr bool only_create =
    !std::is_default_constructible<T>::value && !std::is_constructible<T, std::string>::value &&
    !std::is_constructible<T, std::string_view>::value && !std::is_constructible<T, const char*>::value &&
    std::is_copy_constructible<T>::value &&
    std::is_same<decltype(T::create(std::string_view())), std::optional<T>>::value &&
    std::is_same<decltype(std::declval<const T&>().value()), const std::string&>::value;
static_assert(only_create<m::EmptyMarkerType>, "EmptyMarkerType construction policy");
static_assert(only_create<m::TailType>, "TailType construction policy");
static_assert(only_create<m::CodeType>, "CodeType construction policy");
static_assert(only_create<m::NameType>, "NameType construction policy");
static_assert(only_create<m::LabelType>, "LabelType construction policy");
static_assert(only_create<m::MissionType>, "MissionType construction policy");
static_assert(only_create<m::UnitNameType>, "UnitNameType construction policy");
static_assert(only_create<m::SerialType>, "SerialType construction policy");
static_assert(only_create<m::PinType>, "PinType construction policy");
static_assert(only_create<m::OctalType>, "OctalType construction policy");
static_assert(only_create<m::VisibleType>, "VisibleType construction policy");
static_assert(only_create<m::LaunchType>, "LaunchType construction policy");
// The optional zero-length member is an optional carrier, never a bare string
// and never a bool: present "" and absent stay distinct in the model.
static_assert(std::is_same<decltype(m::BoundedPayloadType::unassignall),
                           std::optional<m::EmptyMarkerType>>::value,
              "UnassignAll is std::optional<EmptyMarkerType>");

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

const std::vector<std::string> kOrder = {
    "Marker", "UnassignAll", "Tail", "Code", "Name", "Label", "Mission", "UnitName", "Serial",
    "Pin", "Octal", "Visible", "Markers", "Names", "Codes", "Pick", "Picks", "Inner"};

const std::map<std::string, std::string> kBase = {
    {"Marker", R"("")"},
    {"Tail", R"("AB12 XY9")"},
    {"Code", R"("aZ09")"},
    {"Names", R"(["a","B"])"},
    {"Pick", R"({"Launch":"ABC"})"},
    {"Inner", R"({"Tail":"        "})"},
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

std::string quoted(const std::string& text) { return "\"" + text + "\""; }

// ---- assertions --------------------------------------------------------------

using Payload = m::BoundedPayloadType;

/// Decode must succeed; the typed value is checked by `inspect`; encoding the
/// decoded value must give exactly `expected` (the input unless stated).
template <typename Inspect>
void accept(const std::string& json, Inspect inspect, const std::string& expected) {
    const auto decoded = sc::decode_BoundedPayloadType(json);
    ++checks;
    if (!decoded.ok()) {
        ++failures;
        std::cerr << "rejected valid document " << json << ": " << decoded.error().path << ": "
                  << decoded.error().message << '\n';
        return;
    }
    inspect(decoded.value());
    const auto encoded = sc::encode_BoundedPayloadType(decoded.value());
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

/// Decode must fail at exactly `path` with a message containing `needle`.
void reject(const std::string& json, const std::string& path, const std::string& needle) {
    const auto decoded = sc::decode_BoundedPayloadType(json);
    ++checks;
    ++rejections;
    if (decoded.ok()) {
        ++failures;
        std::cerr << "accepted invalid document " << json << " (expected " << path << ")\n";
        return;
    }
    if (decoded.error().path != path || decoded.error().message.find(needle) == std::string::npos) {
        ++failures;
        std::cerr << "wrong rejection for " << json << "\n  expected " << path << ": ..." << needle
                  << "...\n  actual   " << decoded.error().path << ": " << decoded.error().message << '\n';
    }
}

const std::string kRoot = "BoundedPayloadType";

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

// ---- suites ------------------------------------------------------------------

/// Zero-length EmptyType shape: required, optional, nested optional, repeated,
/// Choice alternative.
void empty_type() {
    // Required present "" and optional absent.
    accept(doc(), [](const Payload& v) {
        CHECK(v.marker.value().empty());
        CHECK(!v.unassignall.has_value());
        CHECK(!v.inner.unassign.has_value());
        CHECK(v.markers.values().empty());
    });
    // Optional PRESENT "" is distinct from absent and survives the round trip.
    accept(doc({{"UnassignAll", R"("")"}}), [](const Payload& v) {
        CHECK(v.unassignall.has_value());
        CHECK(v.unassignall && v.unassignall->value().empty());
    });
    accept(doc({{"Inner", R"({"Tail":"        ","Unassign":""})"}}), [](const Payload& v) {
        CHECK(v.inner.unassign.has_value() && v.inner.unassign->value().empty());
    });
    // Escaped forms that decode to the empty string do not exist; an escape
    // that decodes to one character is one character.
    for (const char* bad : {R"(" ")", R"("x")", R"("A")", R"("  ")", R"("\t")", R"("\u0000")",
                            R"("\u0020")", R"("\n")", R"("é")"}) {
        reject_member("UnassignAll", bad, "EmptyMarkerType");
        reject_member("Marker", bad, "EmptyMarkerType");
    }
    for (const auto& kind : std::vector<std::pair<std::string, std::string>>{
             {"0", "number"}, {"null", "null"}, {"[]", "array"}, {R"([""])", "array"},
             {"{}", "object"}, {"false", "boolean"}}) {
        reject_kind("UnassignAll", kind.first, kind.second);
        reject_kind("Marker", kind.first, kind.second);
    }
    reject(doc({{"Marker", std::nullopt}}), kRoot, "missing required member \"Marker\"");
    reject(doc({{"Inner", R"({"Tail":"        ","Unassign":" "})"}}), kRoot + ".Inner.Unassign",
           model_check("EmptyMarkerType"));
    reject(doc({{"Inner", R"({"Tail":"        ","Unassign":null})"}}), kRoot + ".Inner.Unassign",
           "expected string, found null");

    // Repetition 0..3 of the zero-length value.
    accept(doc({{"Markers", R"(["","",""])"}}), [](const Payload& v) {
        CHECK(v.markers.values().size() == 3);
        for (const auto& marker : v.markers.values()) {
            CHECK(marker.value().empty());
        }
    });
    // A present empty array is the empty repetition, which encodes as absent.
    accept(doc({{"Markers", "[]"}}), [](const Payload& v) { CHECK(v.markers.values().empty()); }, doc());
    reject(doc({{"Markers", R"(["",""," "])"}}), kRoot + ".Markers[2]", model_check("EmptyMarkerType"));
    reject(doc({{"Markers", R"(["",0])"}}), kRoot + ".Markers[1]", "expected string, found number");
    reject(doc({{"Markers", R"(["","","",""])"}}), kRoot + ".Markers",
           "4 occurrence(s) rejected by the generated sequence constructor");
    reject(doc({{"Markers", R"("")"}}), kRoot + ".Markers", "expected array, found string");
    reject(doc({{"Markers", "null"}}), kRoot + ".Markers", "expected array, found null");

    // Choice alternative.
    accept(doc({{"Pick", R"({"Empty":""})"}}), [](const Payload& v) {
        const auto* empty = std::get_if<m::PickChoice::Empty>(&v.pick.value);
        CHECK(empty != nullptr && empty->value.value().empty());
    });
    reject(doc({{"Pick", R"({"Empty":" "})"}}), kRoot + ".Pick.Empty", model_check("EmptyMarkerType"));
    reject(doc({{"Pick", R"({"Empty":null})"}}), kRoot + ".Pick.Empty", "expected string, found null");
}

/// Fixed-length profiles: exact length, SPACE membership, case, non-ASCII.
void fixed_length() {
    // TailType [A-Z0-9 ]{8}: SPACE is a member, anywhere, and is preserved.
    for (const char* good : {"AB12 XY9", "        ", " AB12XY9", "AB12XY9 ", "ZZZZZZZZ", "00000000"}) {
        accept(doc({{"Tail", quoted(good)}}), [good](const Payload& v) { CHECK(v.tail.value() == good); });
    }
    for (const char* bad : {"AB12XY9", "AB12 XY90", "", "ab12 xy9", "AB12 Xy9", "AB12_XY9", "AB12-XY9",
                            "AB12\\tXY9", "AB12\\u00A0XY", "AB12\\nXY9", "AB12ÉXY", "AB12XY9\\u0000",
                            "ＡB12XY9"}) {
        reject_member("Tail", quoted(bad), "TailType");
    }
    reject(doc({{"Inner", R"({"Tail":"short"})"}}), kRoot + ".Inner.Tail", model_check("TailType"));
    reject(doc({{"Inner", R"({"Unassign":""})"}}), kRoot + ".Inner", "missing required member \"Tail\"");
    reject_kind("Tail", "12345678", "number");
    reject_kind("Tail", R"(["AB12 XY9"])", "array");

    // CodeType [a-zA-Z0-9]{4}: no SPACE, no punctuation.
    for (const char* good : {"aZ09", "0000", "zzzz", "ABCD"}) {
        accept(doc({{"Code", quoted(good)}}), [good](const Payload& v) { CHECK(v.code.value() == good); });
    }
    for (const char* bad : {"aZ0", "aZ09x", "aZ-9", "aZ0 ", " aZ0", "aZ_9", "aZ.9", "aZ0é"}) {
        reject_member("Code", quoted(bad), "CodeType");
    }

    // OctalType [0-7]{4}: a digit sub-range.
    accept(doc({{"Octal", R"("0717")"}}), [](const Payload& v) { CHECK(v.octal && v.octal->value() == "0717"); });
    for (const char* bad : {"0718", "0719", "071", "07170", "07a7"}) {
        reject_member("Octal", quoted(bad), "OctalType");
    }

    // VisibleType [ -~]{6}: every visible ASCII character incl. " and \, by
    // character count. A two-byte UTF-8 character must not satisfy the length.
    accept(doc({{"Visible", R"("a\"b\\c ")"}}), [](const Payload& v) {
        CHECK(v.visible && v.visible->value() == "a\"b\\c ");
    });
    accept(doc({{"Visible", R"("      ")"}}));
    accept(doc({{"Visible", R"(" !~{}|")"}}));
    for (const char* bad : {"abcdé", "abcde\\u007F", "abcde\\t", "abcdefg", "abcde"}) {
        reject_member("Visible", quoted(bad), "VisibleType");
    }
}

/// Bounded-length profiles: off-by-one at both ends, punctuation classes.
void bounded_length() {
    const std::string twenty(20, 'a');
    const std::string twenty_one(21, 'a');
    accept(doc({{"Name", R"("a")"}}));
    accept(doc({{"Name", quoted(twenty)}}), [&twenty](const Payload& v) { CHECK(v.name && v.name->value() == twenty); });
    reject_member("Name", quoted(twenty_one), "NameType");
    reject_member("Name", R"("")", "NameType");
    reject_member("Name", R"("a b")", "NameType");
    reject_member("Name", R"("a-b")", "NameType");

    // LabelType [a-zA-Z0-9 \-_]{1,20}: leading/trailing SPACE preserved,
    // distinct from the trimmed form.
    accept(doc({{"Label", R"(" a-b_C 9 ")"}}), [](const Payload& v) {
        CHECK(v.label && v.label->value() == " a-b_C 9 ");
        CHECK(v.label && v.label->value() != "a-b_C 9");
    });
    accept(doc({{"Label", R"(" ")"}}));
    accept(doc({{"Label", quoted(std::string(20, '_'))}}));
    for (const char* bad : {"", "a.b", "a,b", "a\\tb", "a\\u00e9b"}) {
        reject_member("Label", quoted(bad), "LabelType");
    }
    reject_member("Label", quoted(std::string(21, '-')), "LabelType");

    // MissionType [\-\.,\(\)\?A-Z0-9 ]{1,8}: USMTF punctuation, upper case only.
    accept(doc({{"Mission", R"("A-1.(,)?")"}}));
    accept(doc({{"Mission", R"("?")"}}));
    for (const char* bad : {"a", "A_1", "A/1", "A:1", "A-1.(,)?9", "", "A'1"}) {
        reject_member("Mission", quoted(bad), "MissionType");
    }

    // UnitNameType [a-zA-Z0-9 '\(\).,@;+\-]{1,256}.
    accept(doc({{"UnitName", R"("O'Neil (3rd) Co., @HQ; A+B-1.")"}}), [](const Payload& v) {
        CHECK(v.unitname && v.unitname->value() == "O'Neil (3rd) Co., @HQ; A+B-1.");
    });
    accept(doc({{"UnitName", quoted(std::string(256, 'Z'))}}));
    reject_member("UnitName", quoted(std::string(257, 'Z')), "UnitNameType");
    for (const char* bad : {"a\\\"b", "a/b", "a_b", "a:b", "a?b", "a!b", ""}) {
        reject_member("UnitName", quoted(bad), "UnitNameType");
    }

    // SerialType (USMTF serial class) {1,7}: " and \ are members, so the JSON
    // escapes must decode to exactly those characters and re-encode canonically.
    accept(doc({{"Serial", R"("A\"\\<>&")"}}), [](const Payload& v) {
        CHECK(v.serial && v.serial->value() == "A\"\\<>&");
        CHECK(v.serial && v.serial->value().size() == 6);
    });
    accept(doc({{"Serial", R"("~`|{}[]")"}}));
    // Non-canonical escapes of members decode to the same value and re-encode
    // with only the JSON-mandatory escapes.
    accept(doc({{"Serial", R"("\u0041\u0022")"}}), [](const Payload& v) {
        CHECK(v.serial && v.serial->value() == "A\"");
    }, doc({{"Serial", R"("A\"")"}}));
    for (const char* bad : {"a", "A/B", "A:B", "ABCDEFGH", "", "A\\tB"}) {
        reject_member("Serial", quoted(bad), "SerialType");
    }

    // PinType [0-9]{4,5}: minLength > 1.
    accept(doc({{"Pin", R"("0123")"}}));
    accept(doc({{"Pin", R"("01234")"}}));
    for (const char* bad : {"012", "012345", "012a", "0 23", "０１２３", "-123"}) {
        reject_member("Pin", quoted(bad), "PinType");
    }
    reject_kind("Pin", "1234", "number");
}

/// The "/" document above must have been rejected, not accepted: the solidus
/// is outside the class. (Kept separate so the intent is explicit.)
void serial_solidus() {
    reject_member("Serial", R"("\u0041\u0022\/")", "SerialType");
}

/// Repetition and Choice composition with bounded-ASCII values.
void composition() {
    accept(doc({{"Names", R"(["a","B","c","D"])"}}), [](const Payload& v) {
        CHECK(v.names.values().size() == 4);
        CHECK(v.names.values()[3].value() == "D");
    });
    reject(doc({{"Names", R"(["a"])"}}), kRoot + ".Names",
           "1 occurrence(s) rejected by the generated sequence constructor");
    reject(doc({{"Names", std::nullopt}}), kRoot + ".Names",
           "0 occurrence(s) rejected by the generated sequence constructor");
    reject(doc({{"Names", "[]"}}), kRoot + ".Names", "0 occurrence(s) rejected by the generated sequence constructor");
    reject(doc({{"Names", R"(["a","b","c","d","e"])"}}), kRoot + ".Names",
           "5 occurrence(s) rejected by the generated sequence constructor");
    reject(doc({{"Names", R"(["a","b","c d"])"}}), kRoot + ".Names[2]", model_check("NameType"));
    reject(doc({{"Names", R"(["a",""])"}}), kRoot + ".Names[1]", model_check("NameType"));

    accept(doc({{"Codes", R"(["abcd","EFGH","0000"])"}}), [](const Payload& v) {
        CHECK(v.codes.values().size() == 3);
        CHECK(v.codes.values()[1].value() == "EFGH");
    });
    accept(doc({{"Codes", "[]"}}), [](const Payload& v) { CHECK(v.codes.values().empty()); }, doc());
    reject(doc({{"Codes", R"(["abcd","ab"])"}}), kRoot + ".Codes[1]", model_check("CodeType"));
    reject(doc({{"Codes", R"(["abcd",null])"}}), kRoot + ".Codes[1]", "expected string, found null");

    // Choice alternatives holding bounded-ASCII values.
    accept(doc({{"Pick", R"({"Serial":"\"\\"})"}}), [](const Payload& v) {
        const auto* serial = std::get_if<m::PickChoice::Serial>(&v.pick.value);
        CHECK(serial != nullptr && serial->value.value() == "\"\\");
    });
    accept(doc({{"Pick", R"({"Launch":"Z"})"}}));
    for (const char* bad : {"abc", "", "ABCD", "A1", "A B"}) {
        reject(doc({{"Pick", "{\"Launch\":\"" + std::string(bad) + "\"}"}}), kRoot + ".Pick.Launch",
               model_check("LaunchType"));
    }
    reject(doc({{"Pick", "{}"}}), kRoot + ".Pick", "a Choice requires exactly one selected member");
    reject(doc({{"Pick", R"({"Launch":"A","Empty":""})"}}), kRoot + ".Pick",
           "a Choice requires exactly one selected member");
    reject(doc({{"Pick", R"({"Other":"A"})"}}), kRoot + ".Pick", "unknown member \"Other\"");
    reject(doc({{"Pick", R"("ABC")"}}), kRoot + ".Pick", "expected object, found string");
    reject(doc({{"Pick", std::nullopt}}), kRoot, "missing required member \"Pick\"");

    accept(doc({{"Picks", R"([{"Empty":""},{"Launch":"Z"}])"}}), [](const Payload& v) {
        CHECK(v.picks.values().size() == 2);
        CHECK(std::holds_alternative<m::PickChoice::Empty>(v.picks.values()[0].value));
    });
    reject(doc({{"Picks", R"([{"Empty":""},{"Launch":"z"}])"}}), kRoot + ".Picks[1].Launch",
           model_check("LaunchType"));
    reject(doc({{"Picks", R"([{"Empty":"x"}])"}}), kRoot + ".Picks[0].Empty", model_check("EmptyMarkerType"));
    reject(doc({{"Picks", R"([{"Empty":""},{"Empty":""},{"Empty":""}])"}}), kRoot + ".Picks",
           "3 occurrence(s) rejected by the generated sequence constructor");
}

/// Document-level JSON rules around bounded-ASCII members.
void document_rules() {
    // Duplicate members, also after escape decoding of the key.
    reject(R"({"Marker":"","Marker":"","Tail":"AB12 XY9","Code":"aZ09","Names":["a","B"],"Pick":{"Launch":"ABC"},"Inner":{"Tail":"        "}})",
           "$", "duplicate object member");
    reject(R"({"Marker":"","\u004darker":"","Tail":"AB12 XY9","Code":"aZ09","Names":["a","B"],"Pick":{"Launch":"ABC"},"Inner":{"Tail":"        "}})",
           "$", "duplicate object member");
    reject(doc({{"UnassignAll", R"("")"}}).replace(1, 0, R"("UnassignAll":"",)"), "$", "duplicate object member");
    // Unknown members, including a case variant of a wire name and the
    // global-element envelope.
    reject(doc({{"Extra", R"("")"}}), kRoot, "unknown member \"Extra\"");
    reject(doc({{"unassignAll", R"("")"}}), kRoot, "unknown member \"unassignAll\"");
    reject("{\"BoundedNotice\":" + doc() + "}", kRoot, "unknown member \"BoundedNotice\"");
    // Raw control characters and ill-formed UTF-8 are JSON errors (path "$").
    reject(doc({{"Tail", "\"AB12\tXY9\""}}), "$", "unescaped control character in string");
    reject(doc({{"Label", "\"a\x01\""}}), "$", "unescaped control character in string");
    reject(doc({{"Label", "\"a\xC3\x28\""}}), "$", "ill-formed UTF-8 in string");
    reject(doc({{"Label", "\"a\xC0\xA0\""}}), "$", "ill-formed UTF-8 in string");
    reject(doc({{"Label", R"("\ud800")"}}), "$", "lone high surrogate escape");
    reject(doc() + " x", "$", "unexpected content after the JSON value");
    reject("[" + doc() + "]", kRoot, "expected object, found array");
}

/// Encoding a value built through the model API (not through decode) gives the
/// hand-written canonical text: schema order, absent optionals omitted,
/// present "" written as "", empty repetitions omitted, only the JSON-mandatory
/// escapes.
void encode_from_model() {
    std::vector<m::PickChoice> picks;
    picks.push_back(m::PickChoice{m::PickChoice::Serial{must<m::SerialType>("\\\"")}});
    picks.push_back(m::PickChoice{m::PickChoice::Empty{must<m::EmptyMarkerType>("")}});
    Payload value{
        must<m::EmptyMarkerType>(""),
        must<m::EmptyMarkerType>(""),
        must<m::TailType>(" AB 12  "),
        must<m::CodeType>("Zz90"),
        must<m::NameType>("N"),
        must<m::LabelType>("l -_"),
        must<m::MissionType>("(?)"),
        must<m::UnitNameType>("1st Bn; 'A' Co."),
        must<m::SerialType>("\"<&>\""),
        must<m::PinType>("98765"),
        must<m::OctalType>("7777"),
        must<m::VisibleType>("~\\ \"/ "),
        *m::BoundedVector<m::EmptyMarkerType, 0, 3>::create(
            {must<m::EmptyMarkerType>(""), must<m::EmptyMarkerType>("")}),
        *m::BoundedVector<m::NameType, 2, 4>::create({must<m::NameType>("x"), must<m::NameType>("Y")}),
        *m::UnboundedVector<m::CodeType, 0>::create({}),
        m::PickChoice{m::PickChoice::Launch{must<m::LaunchType>("Q")}},
        *m::BoundedVector<m::PickChoice, 0, 2>::create(std::move(picks)),
        m::InnerType{must<m::TailType>("12345678"), must<m::EmptyMarkerType>("")},
    };
    const std::string expected =
        R"({"Marker":"","UnassignAll":"","Tail":" AB 12  ","Code":"Zz90","Name":"N","Label":"l -_",)"
        R"J("Mission":"(?)","UnitName":"1st Bn; 'A' Co.","Serial":"\"<&>\"","Pin":"98765","Octal":"7777",)J"
        R"("Visible":"~\\ \"/ ","Markers":["",""],"Names":["x","Y"],"Pick":{"Launch":"Q"},)"
        R"("Picks":[{"Serial":"\\\""},{"Empty":""}],"Inner":{"Tail":"12345678","Unassign":""}})";
    const auto encoded = sc::encode_BoundedPayloadType(value);
    CHECK(encoded.ok());
    CHECK(encoded.ok() && encoded.value() == expected);
    if (encoded.ok() && encoded.value() != expected) {
        std::cerr << "  expected " << expected << "\n  actual   " << encoded.value() << '\n';
    }
    // And the canonical text decodes back to the same typed values.
    const auto decoded = sc::decode_BoundedPayloadType(expected);
    CHECK(decoded.ok());
    if (decoded.ok()) {
        const Payload& v = decoded.value();
        CHECK(v.unassignall && v.unassignall->value().empty());
        CHECK(v.tail.value() == " AB 12  ");
        CHECK(v.serial && v.serial->value() == "\"<&>\"");
        CHECK(v.visible && v.visible->value() == "~\\ \"/ ");
        CHECK(v.inner.unassign && v.inner.unassign->value().empty());
        CHECK(v.markers.values().size() == 2);
    }

    // Absent optional zero-length members are omitted, not written as "".
    value.unassignall.reset();
    value.inner.unassign.reset();
    const auto absent = sc::encode_BoundedPayloadType(value);
    CHECK(absent.ok() && absent.value().find("UnassignAll") == std::string::npos);
    CHECK(absent.ok() && absent.value().find("Unassign") == std::string::npos);
    CHECK(absent.ok() && absent.value().find(R"("Inner":{"Tail":"12345678"}})") != std::string::npos);
}

/// The model's own `create` is exercised directly (the codec is required to
/// delegate to it, so these verdicts must match the decode verdicts above).
void model_factories() {
    CHECK(m::EmptyMarkerType::create("").has_value());
    CHECK(!m::EmptyMarkerType::create(" ").has_value());
    CHECK(!m::EmptyMarkerType::create(std::string_view("\0", 1)).has_value());
    CHECK(m::TailType::create("        ").has_value());
    CHECK(!m::TailType::create("abcdefgh").has_value());
    CHECK(m::PinType::create("0000").has_value());
    CHECK(!m::PinType::create("000").has_value());
    CHECK(m::LaunchType::create("ABC").has_value());
    CHECK(!m::LaunchType::create("abc").has_value());
}

}  // namespace

int main() {
    empty_type();
    fixed_length();
    bounded_length();
    serial_solidus();
    composition();
    document_rules();
    encode_from_model();
    model_factories();
    if (failures != 0) {
        std::cerr << failures << " failure(s) of " << checks << " checks\n";
        return 1;
    }
    std::cout << "CPP_OMS_005_BOUNDED_ASCII checks=" << checks << " rejections=" << rejections << '\n';
    return 0;
}
