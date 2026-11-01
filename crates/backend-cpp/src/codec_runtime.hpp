// Generated OMS JSON payload codec (CPP-OMS-001, OMSC-SPC-013 Rev B section 6.1).
//
// Encodes and decodes the PAYLOAD BODY of each selected OMS message as JSON
// text. It owns no transport, envelope, topic, or global-element wrapper.
//
// Dependency strategy: standard library only (C++17). This file embeds a small
// strict RFC 8259 reader and a direct writer in `service_codec::detail`; no
// third-party JSON library is used, because OMS JSON needs behavior generic
// libraries do not guarantee (duplicate-member rejection, exact integer
// tokens, strict UTF-8). The reader and writer are private to this header.
//
// Public API, per unique selected payload type `P` (see the end of this file):
//
//   CodecResult<std::string> encode_P(const model::P&);
//   CodecResult<model::P>    decode_P(std::string_view json_utf8);
//
// Every decoded value is built through the generated model's checked factories
// (`create`), so the model, not this file, decides whether a value inhabits its
// schema type. Failures are returned as `CodecError` values (a schema-derived
// path plus a message); the codec throws nothing itself, apart from the
// standard library's allocation failure.
//
// Decode accepts: strict JSON text; integers only as JSON integer tokens
// (`1`, `-7`; never `1.0`, `1e2`, strings or booleans); members in any order;
// absent or `[]` for a repeated member that allows zero occurrences; and a
// "$type" member only when it names the concrete type being decoded. It
// rejects: duplicate members (after escape decoding), unknown members, null,
// lone surrogate escapes, ill-formed UTF-8, trailing content, a UTF-8 byte
// order mark, and nesting deeper than `detail::max_depth`.
//
// Encode emits members in schema order with no whitespace, omits absent
// optional members and empty repetitions, and never emits "$type". Strings
// are written as UTF-8 with only the JSON-mandatory escapes.
#pragma once

#include <cstddef>
#include <cstdint>
#include <initializer_list>
#include <limits>
#include <optional>
#include <set>
#include <string>
#include <string_view>
#include <utility>
#include <variant>
#include <vector>

@MODEL_INCLUDE@

namespace service_codec {

/// A rejected payload: where (schema-derived path, `$` for JSON syntax) and why.
struct CodecError {
    std::string path;
    std::string message;
};

/// A value or a CodecError. `value()` requires `ok()`.
template <typename T>
class CodecResult {
public:
    static CodecResult success(T value) {
        CodecResult result;
        result.value_.emplace(std::move(value));
        return result;
    }
    static CodecResult failure(CodecError error) {
        CodecResult result;
        result.error_ = std::move(error);
        return result;
    }

    bool ok() const noexcept { return value_.has_value(); }
    explicit operator bool() const noexcept { return ok(); }
    const T& value() const& noexcept { return *value_; }
    T& value() & noexcept { return *value_; }
    T&& value() && noexcept { return std::move(*value_); }
    const CodecError& error() const noexcept { return error_; }

private:
    CodecResult() = default;
    std::optional<T> value_;
    CodecError error_;
};

namespace detail {

constexpr std::size_t max_depth = 256;

// ---- error paths (rendered only when an error occurs) ----------------------

struct Path {
    const Path* parent;
    std::string_view key;
    std::size_t index;
    bool is_index;
};

inline std::string render_path(const Path* path) {
    std::vector<const Path*> chain;
    for (const Path* node = path; node != nullptr; node = node->parent) {
        chain.push_back(node);
    }
    std::string text;
    for (auto it = chain.rbegin(); it != chain.rend(); ++it) {
        const Path* node = *it;
        if (node->is_index) {
            text += '[';
            text += std::to_string(node->index);
            text += ']';
        } else {
            if (!text.empty()) {
                text += '.';
            }
            text.append(node->key.data(), node->key.size());
        }
    }
    return text;
}

inline std::nullopt_t fail(CodecError& error, const Path* path, std::string message) {
    error.path = render_path(path);
    error.message = std::move(message);
    return std::nullopt;
}

inline bool fail_bool(CodecError& error, const Path* path, std::string message) {
    fail(error, path, std::move(message));
    return false;
}

// ---- parsed JSON tree ------------------------------------------------------

struct Member;

struct Value {
    enum class Kind { Null, Boolean, Number, String, Array, Object };
    Kind kind = Kind::Null;
    bool boolean = false;
    std::string text;  // decoded String, or the Number lexeme
    std::vector<Value> array;
    std::vector<Member> object;
};

struct Member {
    std::string key;
    Value value;
};

inline const char* kind_name(Value::Kind kind) {
    switch (kind) {
        case Value::Kind::Null: return "null";
        case Value::Kind::Boolean: return "boolean";
        case Value::Kind::Number: return "number";
        case Value::Kind::String: return "string";
        case Value::Kind::Array: return "array";
        case Value::Kind::Object: return "object";
    }
    return "value";
}

// ---- UTF-8 (RFC 3629): reject overlong forms, surrogates, > U+10FFFF --------

/// Length of the well-formed UTF-8 sequence at `text[at]`, or 0 if ill-formed.
inline std::size_t utf8_sequence(std::string_view text, std::size_t at) {
    const auto byte = [&](std::size_t i) { return static_cast<unsigned>(static_cast<unsigned char>(text[i])); };
    const unsigned lead = byte(at);
    if (lead < 0x80U) {
        return 1;
    }
    std::size_t length = 0;
    unsigned lowest = 0;
    unsigned point = 0;
    if (lead >= 0xC2U && lead <= 0xDFU) {
        length = 2;
        lowest = 0x80U;
        point = lead & 0x1FU;
    } else if (lead >= 0xE0U && lead <= 0xEFU) {
        length = 3;
        lowest = 0x800U;
        point = lead & 0x0FU;
    } else if (lead >= 0xF0U && lead <= 0xF4U) {
        length = 4;
        lowest = 0x10000U;
        point = lead & 0x07U;
    } else {
        return 0;
    }
    if (text.size() - at < length) {
        return 0;
    }
    for (std::size_t i = 1; i < length; ++i) {
        const unsigned next = byte(at + i);
        if ((next & 0xC0U) != 0x80U) {
            return 0;
        }
        point = (point << 6) | (next & 0x3FU);
    }
    if (point < lowest || point > 0x10FFFFU || (point >= 0xD800U && point <= 0xDFFFU)) {
        return 0;
    }
    return length;
}

inline bool is_well_formed_utf8(std::string_view text) {
    std::size_t at = 0;
    while (at < text.size()) {
        const std::size_t length = utf8_sequence(text, at);
        if (length == 0) {
            return false;
        }
        at += length;
    }
    return true;
}

inline void append_utf8(std::string& out, unsigned point) {
    if (point < 0x80U) {
        out += static_cast<char>(point);
    } else if (point < 0x800U) {
        out += static_cast<char>(0xC0U | (point >> 6));
        out += static_cast<char>(0x80U | (point & 0x3FU));
    } else if (point < 0x10000U) {
        out += static_cast<char>(0xE0U | (point >> 12));
        out += static_cast<char>(0x80U | ((point >> 6) & 0x3FU));
        out += static_cast<char>(0x80U | (point & 0x3FU));
    } else {
        out += static_cast<char>(0xF0U | (point >> 18));
        out += static_cast<char>(0x80U | ((point >> 12) & 0x3FU));
        out += static_cast<char>(0x80U | ((point >> 6) & 0x3FU));
        out += static_cast<char>(0x80U | (point & 0x3FU));
    }
}

// ---- strict JSON reader ----------------------------------------------------

class Parser {
public:
    Parser(std::string_view text, CodecError& error) : text_(text), error_(error) {}

    bool parse(Value& out) {
        skip_space();
        if (!parse_value(out, 0)) {
            return false;
        }
        skip_space();
        if (at_ != text_.size()) {
            return syntax("unexpected content after the JSON value");
        }
        return true;
    }

private:
    bool syntax(const std::string& message) {
        error_.path = "$";
        error_.message = message + " at byte " + std::to_string(at_);
        return false;
    }

    void skip_space() {
        while (at_ < text_.size()) {
            const char c = text_[at_];
            if (c != ' ' && c != '\t' && c != '\n' && c != '\r') {
                break;
            }
            ++at_;
        }
    }

    bool parse_value(Value& out, std::size_t depth) {
        if (at_ >= text_.size()) {
            return syntax("unexpected end of input");
        }
        const char c = text_[at_];
        if (c == '{') {
            return parse_object(out, depth);
        }
        if (c == '[') {
            return parse_array(out, depth);
        }
        if (c == '"') {
            out.kind = Value::Kind::String;
            return parse_string(out.text);
        }
        if (c == '-' || (c >= '0' && c <= '9')) {
            out.kind = Value::Kind::Number;
            return parse_number(out.text);
        }
        if (literal("true")) {
            out.kind = Value::Kind::Boolean;
            out.boolean = true;
            return true;
        }
        if (literal("false")) {
            out.kind = Value::Kind::Boolean;
            out.boolean = false;
            return true;
        }
        if (literal("null")) {
            out.kind = Value::Kind::Null;
            return true;
        }
        return syntax("unexpected character");
    }

    bool literal(std::string_view word) {
        if (text_.substr(at_, word.size()) == word) {
            at_ += word.size();
            return true;
        }
        return false;
    }

    bool parse_array(Value& out, std::size_t depth) {
        if (depth >= max_depth) {
            return syntax("nesting is too deep");
        }
        out.kind = Value::Kind::Array;
        ++at_;
        skip_space();
        if (at_ < text_.size() && text_[at_] == ']') {
            ++at_;
            return true;
        }
        for (;;) {
            out.array.emplace_back();
            skip_space();
            if (!parse_value(out.array.back(), depth + 1)) {
                return false;
            }
            skip_space();
            if (at_ >= text_.size()) {
                return syntax("unterminated array");
            }
            if (text_[at_] == ',') {
                ++at_;
                continue;
            }
            if (text_[at_] == ']') {
                ++at_;
                return true;
            }
            return syntax("expected ',' or ']'");
        }
    }

    bool parse_object(Value& out, std::size_t depth) {
        if (depth >= max_depth) {
            return syntax("nesting is too deep");
        }
        out.kind = Value::Kind::Object;
        ++at_;
        skip_space();
        if (at_ < text_.size() && text_[at_] == '}') {
            ++at_;
            return true;
        }
        std::set<std::string> seen;
        for (;;) {
            skip_space();
            if (at_ >= text_.size() || text_[at_] != '"') {
                return syntax("expected a member name string");
            }
            Member member;
            if (!parse_string(member.key)) {
                return false;
            }
            if (!seen.insert(member.key).second) {
                return syntax("duplicate object member");
            }
            skip_space();
            if (at_ >= text_.size() || text_[at_] != ':') {
                return syntax("expected ':'");
            }
            ++at_;
            skip_space();
            if (!parse_value(member.value, depth + 1)) {
                return false;
            }
            out.object.push_back(std::move(member));
            skip_space();
            if (at_ >= text_.size()) {
                return syntax("unterminated object");
            }
            if (text_[at_] == ',') {
                ++at_;
                continue;
            }
            if (text_[at_] == '}') {
                ++at_;
                return true;
            }
            return syntax("expected ',' or '}'");
        }
    }

    bool hex4(unsigned& out) {
        if (text_.size() - at_ < 4) {
            return syntax("truncated \\u escape");
        }
        unsigned point = 0;
        for (std::size_t i = 0; i < 4; ++i) {
            const char c = text_[at_ + i];
            unsigned digit = 0;
            if (c >= '0' && c <= '9') {
                digit = static_cast<unsigned>(c - '0');
            } else if (c >= 'a' && c <= 'f') {
                digit = static_cast<unsigned>(c - 'a') + 10U;
            } else if (c >= 'A' && c <= 'F') {
                digit = static_cast<unsigned>(c - 'A') + 10U;
            } else {
                return syntax("invalid \\u escape");
            }
            point = (point << 4) | digit;
        }
        at_ += 4;
        out = point;
        return true;
    }

    bool parse_string(std::string& out) {
        ++at_;  // opening quote
        for (;;) {
            if (at_ >= text_.size()) {
                return syntax("unterminated string");
            }
            const auto byte = static_cast<unsigned char>(text_[at_]);
            if (byte == '"') {
                ++at_;
                return true;
            }
            if (byte < 0x20U) {
                return syntax("unescaped control character in string");
            }
            if (byte != '\\') {
                const std::size_t length = utf8_sequence(text_, at_);
                if (length == 0) {
                    return syntax("ill-formed UTF-8 in string");
                }
                out.append(text_.data() + at_, length);
                at_ += length;
                continue;
            }
            ++at_;
            if (at_ >= text_.size()) {
                return syntax("unterminated escape");
            }
            const char escape = text_[at_++];
            switch (escape) {
                case '"': out += '"'; break;
                case '\\': out += '\\'; break;
                case '/': out += '/'; break;
                case 'b': out += '\b'; break;
                case 'f': out += '\f'; break;
                case 'n': out += '\n'; break;
                case 'r': out += '\r'; break;
                case 't': out += '\t'; break;
                case 'u': {
                    unsigned point = 0;
                    if (!hex4(point)) {
                        return false;
                    }
                    if (point >= 0xDC00U && point <= 0xDFFFU) {
                        return syntax("lone low surrogate escape");
                    }
                    if (point >= 0xD800U && point <= 0xDBFFU) {
                        if (text_.substr(at_, 2) != "\\u") {
                            return syntax("lone high surrogate escape");
                        }
                        at_ += 2;
                        unsigned low = 0;
                        if (!hex4(low)) {
                            return false;
                        }
                        if (low < 0xDC00U || low > 0xDFFFU) {
                            return syntax("high surrogate escape without a low surrogate");
                        }
                        point = 0x10000U + ((point - 0xD800U) << 10) + (low - 0xDC00U);
                    }
                    append_utf8(out, point);
                    break;
                }
                default:
                    return syntax("invalid escape");
            }
        }
    }

    bool digits() {
        const std::size_t start = at_;
        while (at_ < text_.size() && text_[at_] >= '0' && text_[at_] <= '9') {
            ++at_;
        }
        return at_ > start;
    }

    bool parse_number(std::string& out) {
        const std::size_t start = at_;
        if (text_[at_] == '-') {
            ++at_;
        }
        if (at_ >= text_.size()) {
            return syntax("incomplete number");
        }
        if (text_[at_] == '0') {
            ++at_;
        } else if (!digits()) {
            return syntax("invalid number");
        }
        if (at_ < text_.size() && text_[at_] == '.') {
            ++at_;
            if (!digits()) {
                return syntax("invalid number fraction");
            }
        }
        if (at_ < text_.size() && (text_[at_] == 'e' || text_[at_] == 'E')) {
            ++at_;
            if (at_ < text_.size() && (text_[at_] == '+' || text_[at_] == '-')) {
                ++at_;
            }
            if (!digits()) {
                return syntax("invalid number exponent");
            }
        }
        out.assign(text_.data() + start, at_ - start);
        return true;
    }

    std::string_view text_;
    CodecError& error_;
    std::size_t at_ = 0;
};

inline bool parse_json(std::string_view text, Value& out, CodecError& error) {
    return Parser(text, error).parse(out);
}

// ---- decode helpers --------------------------------------------------------

inline bool expect_kind(const Value& value, Value::Kind kind, CodecError& error, const Path* path) {
    if (value.kind == kind) {
        return true;
    }
    return fail_bool(error, path,
                     std::string("expected ") + kind_name(kind) + ", found " + kind_name(value.kind));
}

inline const Value* find_member(const Value& object, std::string_view key) {
    for (const Member& member : object.object) {
        if (member.key == key) {
            return &member.value;
        }
    }
    return nullptr;
}

/// Reject unknown members. "$type" is accepted only when it names exactly the
/// concrete type being decoded (any other would silently lose content).
inline bool check_members(const Value& object, CodecError& error, const Path* path,
                          std::string_view own_type,
                          std::initializer_list<std::string_view> allowed) {
    for (const Member& member : object.object) {
        if (member.key == "$type") {
            if (member.value.kind != Value::Kind::String || member.value.text != own_type) {
                return fail_bool(error, path, "\"$type\" does not name the concrete type being decoded");
            }
            continue;
        }
        bool known = false;
        for (const std::string_view name : allowed) {
            if (member.key == name) {
                known = true;
                break;
            }
        }
        if (!known) {
            return fail_bool(error, path, "unknown member \"" + member.key + "\"");
        }
    }
    return true;
}

inline std::optional<bool> get_bool(const Value& value, CodecError& error, const Path* path) {
    if (!expect_kind(value, Value::Kind::Boolean, error, path)) {
        return std::nullopt;
    }
    return value.boolean;
}

inline const std::string* get_text(const Value& value, CodecError& error, const Path* path) {
    if (!expect_kind(value, Value::Kind::String, error, path)) {
        return nullptr;
    }
    return &value.text;
}

inline std::optional<std::string> get_string(const Value& value, CodecError& error, const Path* path) {
    const std::string* text = get_text(value, error, path);
    if (text == nullptr) {
        return std::nullopt;
    }
    return *text;
}

/// Magnitude of an integer token; the token must be an integer lexeme
/// (`-?digits`), never a fraction or exponent form.
inline bool integer_lexeme(const Value& value, bool& negative, std::uint64_t& magnitude,
                           CodecError& error, const Path* path) {
    if (!expect_kind(value, Value::Kind::Number, error, path)) {
        return false;
    }
    std::size_t at = 0;
    negative = !value.text.empty() && value.text[0] == '-';
    if (negative) {
        at = 1;
    }
    magnitude = 0;
    for (; at < value.text.size(); ++at) {
        const char c = value.text[at];
        if (c < '0' || c > '9') {
            return fail_bool(error, path, "expected an integer, found a fraction or exponent form");
        }
        const auto digit = static_cast<std::uint64_t>(c - '0');
        if (magnitude > (std::numeric_limits<std::uint64_t>::max() - digit) / 10U) {
            return fail_bool(error, path, "integer is out of range");
        }
        magnitude = magnitude * 10U + digit;
    }
    return true;
}

inline std::optional<std::int64_t> get_i64(const Value& value, CodecError& error, const Path* path) {
    bool negative = false;
    std::uint64_t magnitude = 0;
    if (!integer_lexeme(value, negative, magnitude, error, path)) {
        return std::nullopt;
    }
    constexpr std::uint64_t limit = static_cast<std::uint64_t>(std::numeric_limits<std::int64_t>::max());
    if (!negative) {
        if (magnitude > limit) {
            return fail(error, path, "integer is out of range");
        }
        return static_cast<std::int64_t>(magnitude);
    }
    if (magnitude > limit + 1U) {
        return fail(error, path, "integer is out of range");
    }
    if (magnitude == limit + 1U) {
        return std::numeric_limits<std::int64_t>::min();
    }
    return -static_cast<std::int64_t>(magnitude);
}

inline std::optional<std::uint64_t> get_u64(const Value& value, CodecError& error, const Path* path) {
    bool negative = false;
    std::uint64_t magnitude = 0;
    if (!integer_lexeme(value, negative, magnitude, error, path)) {
        return std::nullopt;
    }
    if (negative && magnitude != 0) {
        return fail(error, path, "integer is out of range");
    }
    return magnitude;
}

/// A checked integral carrier (`Carrier::create(std::int64_t)`).
template <typename Carrier>
std::optional<Carrier> get_checked_i64(const Value& value, CodecError& error, const Path* path) {
    const std::optional<std::int64_t> number = get_i64(value, error, path);
    if (!number) {
        return std::nullopt;
    }
    std::optional<Carrier> checked = Carrier::create(*number);
    if (!checked) {
        return fail(error, path, "value rejected by the generated model's domain check");
    }
    return checked;
}

template <typename Carrier>
std::optional<Carrier> get_checked_u64(const Value& value, CodecError& error, const Path* path) {
    const std::optional<std::uint64_t> number = get_u64(value, error, path);
    if (!number) {
        return std::nullopt;
    }
    std::optional<Carrier> checked = Carrier::create(*number);
    if (!checked) {
        return fail(error, path, "value rejected by the generated model's domain check");
    }
    return checked;
}

/// A checked text carrier (`Carrier::create(std::string_view)`): a constrained
/// String profile, DateTime or Duration. The generated model alone decides
/// membership; `what` names the schema type for the error message.
template <typename Carrier>
std::optional<Carrier> get_checked_text(const Value& value, CodecError& error, const Path* path,
                                        std::string_view what) {
    const std::string* text = get_text(value, error, path);
    if (text == nullptr) {
        return std::nullopt;
    }
    std::optional<Carrier> checked = Carrier::create(std::string_view(*text));
    if (!checked) {
        return fail(error, path,
                    "value rejected by the generated model's " + std::string(what) + " check");
    }
    return checked;
}

/// XML Schema 2E `hexBinary` lexical parse of one JSON string: first the fixed
/// `whiteSpace=collapse` normalization (TAB/LF/CR become SPACE, runs collapse,
/// leading/trailing SPACE removed), then pairs of case-insensitive ASCII
/// hexadecimal digits. An internal SPACE survives collapse and is rejected.
inline std::optional<std::vector<std::uint8_t>> get_hex_binary(const Value& value, CodecError& error,
                                                               const Path* path) {
    const std::string* text = get_text(value, error, path);
    if (text == nullptr) {
        return std::nullopt;
    }
    std::string collapsed;
    collapsed.reserve(text->size());
    bool pending_space = false;
    for (const char c : *text) {
        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
            pending_space = !collapsed.empty();
        } else {
            if (pending_space) {
                collapsed += ' ';
                pending_space = false;
            }
            collapsed += c;
        }
    }
    const auto nibble = [](char c) -> int {
        if (c >= '0' && c <= '9') {
            return c - '0';
        }
        if (c >= 'a' && c <= 'f') {
            return c - 'a' + 10;
        }
        if (c >= 'A' && c <= 'F') {
            return c - 'A' + 10;
        }
        return -1;
    };
    for (const char c : collapsed) {
        if (nibble(c) < 0) {
            return fail(error, path, "not hexBinary: only hexadecimal digits [0-9A-Fa-f] are allowed");
        }
    }
    if (collapsed.size() % 2 != 0) {
        return fail(error, path, "not hexBinary: odd number of hexadecimal digits");
    }
    std::vector<std::uint8_t> octets;
    octets.reserve(collapsed.size() / 2);
    for (std::size_t i = 0; i < collapsed.size(); i += 2) {
        octets.push_back(static_cast<std::uint8_t>((nibble(collapsed[i]) << 4) | nibble(collapsed[i + 1])));
    }
    return octets;
}

/// The one selected member of a Choice object (`"$type"` excluded). Zero or
/// several selected members are rejected.
inline const Member* find_choice_member(const Value& object, CodecError& error, const Path* path) {
    const Member* selected = nullptr;
    for (const Member& member : object.object) {
        if (member.key == "$type") {
            continue;
        }
        if (selected != nullptr) {
            fail(error, path, "a Choice requires exactly one selected member");
            return nullptr;
        }
        selected = &member;
    }
    if (selected == nullptr) {
        fail(error, path, "a Choice requires exactly one selected member");
    }
    return selected;
}

inline std::nullopt_t fail_cardinality(CodecError& error, const Path* path, std::size_t count) {
    return fail(error, path,
                std::to_string(count) + " occurrence(s) rejected by the generated sequence constructor");
}

// ---- encode helpers --------------------------------------------------------

inline void put_separator(std::string& out, bool& first) {
    if (!first) {
        out += ',';
    }
    first = false;
}

inline bool put_bool(std::string& out, bool value) {
    out += value ? "true" : "false";
    return true;
}

inline bool put_i64(std::string& out, std::int64_t value) {
    out += std::to_string(value);
    return true;
}

inline bool put_u64(std::string& out, std::uint64_t value) {
    out += std::to_string(value);
    return true;
}

/// A JSON string: UTF-8 passes through; only `"`, `\` and control characters
/// are escaped. Ill-formed UTF-8 cannot be represented and is rejected.
inline bool put_string(std::string& out, std::string_view text, CodecError& error, const Path* path) {
    if (!is_well_formed_utf8(text)) {
        return fail_bool(error, path, "String is not well-formed UTF-8");
    }
    static const char hex[] = "0123456789abcdef";
    out += '"';
    for (const char c : text) {
        const auto byte = static_cast<unsigned char>(c);
        if (c == '"') {
            out += "\\\"";
        } else if (c == '\\') {
            out += "\\\\";
        } else if (c == '\b') {
            out += "\\b";
        } else if (c == '\f') {
            out += "\\f";
        } else if (c == '\n') {
            out += "\\n";
        } else if (c == '\r') {
            out += "\\r";
        } else if (c == '\t') {
            out += "\\t";
        } else if (byte < 0x20U) {
            out += "\\u00";
            out += hex[byte >> 4];
            out += hex[byte & 0x0FU];
        } else {
            out += c;
        }
    }
    out += '"';
    return true;
}

/// XML Schema 2E 3.2.15.2 canonical `hexBinary`: two UPPERCASE hexadecimal
/// digits per octet, no separator, written as a JSON string.
inline bool put_hex_binary(std::string& out, const std::vector<std::uint8_t>& octets) {
    static const char digits[] = "0123456789ABCDEF";
    out += '"';
    for (const std::uint8_t octet : octets) {
        out += digits[octet >> 4U];
        out += digits[octet & 0x0FU];
    }
    out += '"';
    return true;
}

/// Generated constants that are already valid JSON text (member keys).
inline void put_raw(std::string& out, std::string_view text) { out.append(text.data(), text.size()); }
