// CPP-OMS-006 consumer: every pinned Task 059 structured-ASCII row through
// the GENERATED C++ model and service_codec headers for the synthetic fixture
// tests/fixtures/cpp-codec/structured-ascii (27 rows under non-UCI names).
//
//   structured_ascii_rows_consumer    -> CPP_OMS_006_STRUCTURED_ROWS rows=27 checks=N rejections=M
//
// For each row, the hand-written valid samples must decode to exactly that
// text, re-encode to exactly the input document, and be accepted by the
// model's own `create`. The hand-written invalid samples must be rejected at
// exactly `StructuredRowsPayload.<Member>` with the model-check message, and
// refused by `create` too, so codec and model agree. Wrong JSON kinds are
// rejected at the same path. The verdicts were derived from each XSD pattern
// and its length facets by hand, not from the generated code.
#include <cstddef>
#include <cstdlib>
#include <initializer_list>
#include <iostream>
#include <map>
#include <optional>
#include <set>
#include <string>
#include <string_view>
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

template <typename T>
T must(std::string_view text, int line = __builtin_LINE()) {
    std::optional<T> created = T::create(text);
    if (!created) {
        std::cerr << "test setup (line " << line << "): a generated factory rejected its argument\n";
        std::exit(2);
    }
    return *created;
}

using Payload = m::StructuredRowsPayload;
const std::string kRoot = "StructuredRowsPayload";

const std::vector<std::string> kOrder = {
    "Prf", "Cycle", "Seno", "File", "Imo", "TrackOne", "Model", "TrackSixteen", "Meter", "Designator",
    "Rating", "Scene", "Flight", "Authority", "Reason", "Downgrade", "Sequence", "FileClass", "Sortie",
    "Digit", "DefenseArea", "Surrogate", "Suffix", "Octal", "Identity", "PacketClass", "Unit", "Files",
    "Sorties", "Pick"};

/// The minimal document (required File and Sorties) with members replaced or
/// added, written in schema order. `nullopt` removes a member.
std::string doc(std::initializer_list<std::pair<std::string, std::optional<std::string>>> changes = {}) {
    std::map<std::string, std::string> members = {{"File", R"("a.b")"}, {"Sorties", R"(["A1"])"}};
    for (const auto& change : changes) {
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
        if (found != members.end()) {
            out += (first ? "\"" : ",\"") + key + "\":" + found->second;
            first = false;
        }
    }
    return out + "}";
}

std::string quoted(const std::string& text) { return "\"" + text + "\""; }

std::string model_check(const std::string& type) {
    return "value rejected by the generated model's " + type + " check";
}

void reject(const std::string& json, const std::string& path, const std::string& message) {
    const auto decoded = sc::decode_StructuredRowsPayload(json);
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

/// Decode must succeed, `inspect` sees the typed value, and re-encoding must
/// give exactly `expected`.
template <typename Inspect>
void accept(const std::string& json, Inspect inspect, const std::string& expected) {
    const auto decoded = sc::decode_StructuredRowsPayload(json);
    ++checks;
    if (!decoded.ok()) {
        ++failures;
        std::cerr << "rejected valid document " << json << ": " << decoded.error().path << ": "
                  << decoded.error().message << '\n';
        return;
    }
    inspect(decoded.value());
    const auto encoded = sc::encode_StructuredRowsPayload(decoded.value());
    ++checks;
    if (!encoded.ok() || encoded.value() != expected) {
        ++failures;
        std::cerr << "re-encoding mismatch for " << json << "\n  expected " << expected << "\n  actual   "
                  << (encoded.ok() ? encoded.value() : encoded.error().path + ": " + encoded.error().message)
                  << '\n';
    }
}

std::set<std::string> rows_seen;

/// One optional row member: valid and invalid raw (escape-free) samples, and
/// JSON-escaped samples that must decode to a rejected value.
template <typename T>
void row(const std::string& key, const std::string& type, std::optional<T> Payload::*member,
         std::initializer_list<const char*> valid, std::initializer_list<const char*> invalid,
         std::initializer_list<const char*> invalid_escaped = {}) {
    rows_seen.insert(type);
    for (const char* text : valid) {
        CHECK(T::create(text).has_value());
        const std::string json = doc({{key, quoted(text)}});
        accept(json, [&](const Payload& v) { CHECK((v.*member) && (v.*member)->value() == text); }, json);
    }
    for (const char* text : invalid) {
        CHECK(!T::create(text).has_value());
        reject(doc({{key, quoted(text)}}), kRoot + "." + key, model_check(type));
    }
    for (const char* escaped : invalid_escaped) {
        reject(doc({{key, quoted(escaped)}}), kRoot + "." + key, model_check(type));
    }
    reject(doc({{key, "17"}}), kRoot + "." + key, "expected string, found number");
    reject(doc({{key, "null"}}), kRoot + "." + key, "expected string, found null");
    reject(doc({{key, R"(["x"])"}}), kRoot + "." + key, "expected string, found array");
}

void all_rows() {
    // 1?[1-7][1-8]{2}, minLength 3, maxLength 4 (length-guided optional prefix).
    row<m::PrfCodeType>("Prf", "PrfCodeType", &Payload::prf,
                        {"111", "178", "188", "788", "1111", "1178", "1788", "1711"},
                        {"189", "2778", "17", "11788", "078", "1078", "179", "1 78", "", "7111", "1709"});
    // [A-Z0-9]{2}-[0-9]{4}[TE]?, minLength 7, maxLength 8.
    row<m::CycleNumberType>("Cycle", "CycleNumberType", &Payload::cycle,
                            {"A1-2345", "ZZ-0000T", "00-9999E", "9Z-1234"},
                            {"A1-2345X", "a1-2345", "A1_2345", "A1-234", "A1-2345TE", "A1-2345t", "A1-23456",
                             "A12345T", "-A1-2345", "A1-2345 "});
    // [A-Z][IRS][0-9]{3}, length 5.
    row<m::SenoType>("Seno", "SenoType", &Payload::seno, {"AI123", "ZS000", "QR999"},
                     {"AX123", "aI123", "AI12", "AI1234", "1I123", "AI12A", "Ai123"});
    // IMO[0-9]{7}, length 10.
    row<m::ImoNumberType>("Imo", "ImoNumberType", &Payload::imo, {"IMO1234567", "IMO0000000"},
                          {"IMO123456", "IMO12345678", "imo1234567", "IMX1234567", "IMO123456A", "XIMO123456"});
    // [AEGHJKLM]{2}[0-7]{3}, length 5.
    row<m::TrackOneType>("TrackOne", "TrackOneType", &Payload::trackone,
                         {"AE017", "MM777", "GH000", "JK123", "LA456"},
                         {"AB017", "AE018", "ae017", "AI017", "AE01", "AE0177", "FE017", "NE017"});
    // [A-Za-z0-9]{1,4}, maxLength 4 only.
    row<m::ModelCodeType>("Model", "ModelCodeType", &Payload::model, {"a", "aZ09", "Z9"},
                          {"", "abcde", "a-b", "a b", "a.b"});
    // [A-HJ-NP-Z0-7]{2}[0-7]{3}, length 5 (no I, no O; digits only 0-7).
    row<m::TrackSixteenType>("TrackSixteen", "TrackSixteenType", &Payload::tracksixteen,
                             {"A0777", "Z7000", "07123", "HJ012", "NP345"},
                             {"I0777", "O0777", "A8777", "AI123", "a0777", "A07778", "89123", "A0778"});
    // [m], length 1.
    row<m::MeterLetterType>("Meter", "MeterLetterType", &Payload::meter, {"m"}, {"M", "mm", "", "n", " "});
    // [a-zA-Z0-9 \-_]{1,16}, maxLength 16.
    row<m::DesignatorType>("Designator", "DesignatorType", &Payload::designator,
                           {"a b-c_d", " ", "abcdefghijklmnop", "-", "_", "  lead trail  "},
                           {"", "abcdefghijklmnopq", "a.b", "a/b", "a+b", "é"},
                           {"a\\tb", "a\\u00A0b", "\\u0000"});
    // [0-9]\.[0-9], length 3.
    row<m::RatingType>("Rating", "RatingType", &Payload::rating, {"1.5", "0.0", "9.9"},
                       {"1,5", "15", "1.55", ".55", "a.5", "1.a", "1-5", "1. "});
    // [0-9 ], length 1.
    row<m::SceneSourceType>("Scene", "SceneSourceType", &Payload::scene, {"7", " ", "0"}, {"a", "", "77", "-"},
                            {"\\t", "\\u00A0"});
    // [A-Z0-9][0-9], length 2.
    row<m::FlightNumberType>("Flight", "FlightNumberType", &Payload::flight, {"A1", "99", "Z0"},
                             {"1A", "a1", "AA", "A", "A12"});
    // [ODM ], length 1.
    row<m::AuthorityMethodType>("Authority", "AuthorityMethodType", &Payload::authority, {"O", "D", "M", " "},
                                {"o", "X", "", "OD", "0"}, {"\\t"});
    // [A-H ], length 1.
    row<m::ReasonCodeType>("Reason", "ReasonCodeType", &Payload::reason, {"A", "H", " ", "D"},
                           {"I", "a", "", "AB", "@"});
    // [SCR ], length 1.
    row<m::DowngradeCodeType>("Downgrade", "DowngradeCodeType", &Payload::downgrade, {"S", "C", "R", " "},
                              {"T", "s", "", "SC"});
    // [1-6 ], length 1.
    row<m::SequenceDigitType>("Sequence", "SequenceDigitType", &Payload::sequence, {"1", "6", " "},
                              {"0", "7", "", "12"});
    // [TSCRU], length 1 (no SPACE).
    row<m::FileClassType>("FileClass", "FileClassType", &Payload::fileclass, {"T", "S", "C", "R", "U"},
                          {" ", "A", "t", "", "TS", "V", "B"});
    // ([A-Z0-9]{2}), length 2: the parentheses group, they are not literals.
    row<m::SortieType>("Sortie", "SortieType", &Payload::sortie, {"A1", "ZZ", "00"},
                       {"(A1)", "a1", "A", "A12", "A-", "()"});
    // [0-9], length 1.
    row<m::DigitType>("Digit", "DigitType", &Payload::digit, {"0", "9"}, {"a", "10", "", "\xD9\xA3"});
    // [A-Z]{2}[0][0-9]{2}, length 5.
    row<m::DefenseAreaType>("DefenseArea", "DefenseAreaType", &Payload::defensearea, {"AB012", "ZZ099", "AA000"},
                            {"AB112", "AB01", "ab012", "A1012", "AB0123", "ABO12"});
    // [a-zA-Z0-9]{5}[0-9]{9}, maxLength 14.
    row<m::SurrogateKeyType>("Surrogate", "SurrogateKeyType", &Payload::surrogate,
                             {"abcde123456789", "ABCDE000000000", "abcd1234567890"},
                             {"abcdef12345678", "abcde12345678", "abcde1234567890", "abcde12345678a",
                              "abc-e123456789", ""});
    // [A-Z]{2}[0-9]{3}, length 5.
    row<m::SuffixCodeType>("Suffix", "SuffixCodeType", &Payload::suffix, {"AB123", "ZZ000"},
                           {"A1123", "AB12", "ab123", "AB1234", "AB12C"});
    // [0-7]+, minLength 1, maxLength 16.
    row<m::OctalCodeType>("Octal", "OctalCodeType", &Payload::octal, {"0", "7777777777777777", "01234567"},
                          {"", "8", "77777777777777777", "0 1", "9"});
    // [a-zA-Z0-9]+, minLength 1, maxLength 10.
    row<m::IdentityCodeType>("Identity", "IdentityCodeType", &Payload::identity, {"a", "abcdefghij", "A1b2C3"},
                             {"", "abcdefghijk", "a b", "a-b", "a_b"});
    // [1-5], length 1.
    row<m::PacketClassType>("PacketClass", "PacketClassType", &Payload::packetclass, {"1", "5", "3"},
                            {"0", "6", "", "12", " "});
    // [A-Z]{2}[ABCDEGJMNSX][A-Z]{2}[0-9]{5}, length 10.
    row<m::UnitCodeType>("Unit", "UnitCodeType", &Payload::unit, {"AAXBB12345", "ZZAZZ00000", "QQSRR99999"},
                         {"AAFBB12345", "AAXBB1234A", "AAXBB1234", "aaXBB12345", "AAxBB12345", "AAXB112345",
                          "AAXBB123456", "AAHBB12345", "AAOBB12345"});
}

/// FileNameType as the REQUIRED member File and the UNBOUNDED member Files;
/// SortieType as the 1..2 member Sorties; the Choice Pick.
void shapes() {
    rows_seen.insert("FileNameType");
    for (const char* good : {"a.b", "report.tar.bz2", "x..", "A-_.z"}) {
        CHECK(m::FileNameType::create(good).has_value());
        const std::string json = doc({{"File", quoted(good)}});
        accept(json, [good](const Payload& v) { CHECK(v.file.value() == good); }, json);
    }
    for (const char* bad : {"report.", ".tar", "report", "re port.txt", "a/b.c", ""}) {
        CHECK(!m::FileNameType::create(bad).has_value());
        reject(doc({{"File", quoted(bad)}}), kRoot + ".File", model_check("FileNameType"));
    }
    reject(doc({{"File", std::nullopt}}), kRoot, "missing required member \"File\"");
    reject(doc({{"File", "null"}}), kRoot + ".File", "expected string, found null");

    accept(doc({{"Files", R"(["a.b","c.d.e"])"}}), [](const Payload& v) { CHECK(v.files.values().size() == 2); },
           doc({{"Files", R"(["a.b","c.d.e"])"}}));
    accept(doc({{"Files", "[]"}}), [](const Payload& v) { CHECK(v.files.values().empty()); }, doc());
    reject(doc({{"Files", R"(["a.b","c."])"}}), kRoot + ".Files[1]", model_check("FileNameType"));

    accept(doc({{"Sorties", R"(["A1","B2"])"}}), [](const Payload& v) { CHECK(v.sorties.values().size() == 2); },
           doc({{"Sorties", R"(["A1","B2"])"}}));
    reject(doc({{"Sorties", R"(["A1","b2"])"}}), kRoot + ".Sorties[1]", model_check("SortieType"));
    reject(doc({{"Sorties", R"(["A1","B2","C3"])"}}), kRoot + ".Sorties",
           "3 occurrence(s) rejected by the generated sequence constructor");
    reject(doc({{"Sorties", "[]"}}), kRoot + ".Sorties",
           "0 occurrence(s) rejected by the generated sequence constructor");
    // An absent repeated member is the empty repetition, which the 1..2
    // sequence constructor refuses (existing codec behavior).
    reject(doc({{"Sorties", std::nullopt}}), kRoot + ".Sorties",
           "0 occurrence(s) rejected by the generated sequence constructor");

    accept(doc({{"Pick", R"({"Unit":"AAXBB12345"})"}}), [](const Payload& v) {
        CHECK(v.pick && std::holds_alternative<m::RowPick::Unit>(v.pick->value));
    }, doc({{"Pick", R"({"Unit":"AAXBB12345"})"}}));
    accept(doc({{"Pick", R"({"Rating":"1.5"})"}}), [](const Payload& v) {
        const auto* rating = v.pick ? std::get_if<m::RowPick::Rating>(&v.pick->value) : nullptr;
        CHECK(rating != nullptr && rating->value.value() == "1.5");
    }, doc({{"Pick", R"({"Rating":"1.5"})"}}));
    reject(doc({{"Pick", R"({"Rating":"15"})"}}), kRoot + ".Pick.Rating", model_check("RatingType"));
    reject(doc({{"Pick", R"({"Unit":"1.5"})"}}), kRoot + ".Pick.Unit", model_check("UnitCodeType"));
    reject(doc({{"Pick", "{}"}}), kRoot + ".Pick", "a Choice requires exactly one selected member");

    // An escaped SPACE is a SPACE (canonical output is the literal character).
    accept(doc({{"Authority", R"("\u0020")"}}),
           [](const Payload& v) { CHECK(v.authority && v.authority->value() == " "); },
           doc({{"Authority", R"(" ")"}}));
}

/// Every row set at once, built through the model API, encodes to exactly the
/// hand-written canonical text, and decodes back.
void encode_all() {
    Payload value{
        must<m::PrfCodeType>("1178"),
        must<m::CycleNumberType>("A1-2345T"),
        must<m::SenoType>("AS001"),
        must<m::FileNameType>("report.tar.bz2"),
        must<m::ImoNumberType>("IMO7654321"),
        must<m::TrackOneType>("LM765"),
        must<m::ModelCodeType>("F35"),
        must<m::TrackSixteenType>("7Z012"),
        must<m::MeterLetterType>("m"),
        must<m::DesignatorType>(" sensor-1_a "),
        must<m::RatingType>("4.5"),
        must<m::SceneSourceType>(" "),
        must<m::FlightNumberType>("Q7"),
        must<m::AuthorityMethodType>("M"),
        must<m::ReasonCodeType>("G"),
        must<m::DowngradeCodeType>(" "),
        must<m::SequenceDigitType>("6"),
        must<m::FileClassType>("U"),
        must<m::SortieType>("0Z"),
        must<m::DigitType>("3"),
        must<m::DefenseAreaType>("XY042"),
        must<m::SurrogateKeyType>("Zz9y8000000001"),
        must<m::SuffixCodeType>("QA100"),
        must<m::OctalCodeType>("0755"),
        must<m::IdentityCodeType>("Rome01"),
        must<m::PacketClassType>("4"),
        must<m::UnitCodeType>("ABJCD54321"),
        *m::UnboundedVector<m::FileNameType, 0>::create({must<m::FileNameType>("a.b")}),
        *m::BoundedVector<m::SortieType, 1, 2>::create({must<m::SortieType>("A1"), must<m::SortieType>("B2")}),
        m::RowPick{m::RowPick::Rating{must<m::RatingType>("0.1")}},
    };
    const std::string expected =
        R"({"Prf":"1178","Cycle":"A1-2345T","Seno":"AS001","File":"report.tar.bz2","Imo":"IMO7654321",)"
        R"("TrackOne":"LM765","Model":"F35","TrackSixteen":"7Z012","Meter":"m","Designator":" sensor-1_a ",)"
        R"("Rating":"4.5","Scene":" ","Flight":"Q7","Authority":"M","Reason":"G","Downgrade":" ",)"
        R"("Sequence":"6","FileClass":"U","Sortie":"0Z","Digit":"3","DefenseArea":"XY042",)"
        R"("Surrogate":"Zz9y8000000001","Suffix":"QA100","Octal":"0755","Identity":"Rome01",)"
        R"("PacketClass":"4","Unit":"ABJCD54321","Files":["a.b"],"Sorties":["A1","B2"],"Pick":{"Rating":"0.1"}})";
    const auto encoded = sc::encode_StructuredRowsPayload(value);
    CHECK(encoded.ok() && encoded.value() == expected);
    if (encoded.ok() && encoded.value() != expected) {
        std::cerr << "  expected " << expected << "\n  actual   " << encoded.value() << '\n';
    }
    accept(expected, [](const Payload& v) {
        CHECK(v.designator && v.designator->value() == " sensor-1_a ");
        CHECK(v.surrogate && v.surrogate->value() == "Zz9y8000000001");
    }, expected);
}

}  // namespace

int main() {
    all_rows();
    shapes();
    encode_all();
    // 26 optional row members plus FileNameType (File / Files): all 27 rows.
    CHECK(rows_seen.size() == 27);
    if (failures != 0) {
        std::cerr << failures << " failure(s) of " << checks << " checks\n";
        return 1;
    }
    std::cout << "CPP_OMS_006_STRUCTURED_ROWS rows=" << rows_seen.size() << " checks=" << checks
              << " rejections=" << rejections << '\n';
    return 0;
}
