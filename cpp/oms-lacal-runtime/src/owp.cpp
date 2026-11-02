// CPP-OMS-003: OWP 1.0 framing and global-element envelope. See owp.hpp.
#include "ams_gra/lacal/owp.hpp"

#include <array>
#include <cstdint>
#include <utility>

namespace ams_gra::lacal::owp {

namespace {

constexpr bool is_delimiter(char c) noexcept { return c == ' ' || c == '\t'; }

constexpr bool is_identifier_char(char c) noexcept {
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '_' ||
           c == '-' || c == '.';
}

/// Splits off the first delimiter-separated token; `rest` starts at the next
/// non-delimiter (the Sleet/OWP tokenisation).
bool split_token(std::string_view text, std::string_view& token, std::string_view& rest) {
    std::size_t start = 0;
    while (start < text.size() && is_delimiter(text[start])) {
        ++start;
    }
    if (start == text.size()) {
        return false;
    }
    std::size_t end = start;
    while (end < text.size() && !is_delimiter(text[end])) {
        ++end;
    }
    token = text.substr(start, end - start);
    std::size_t next = end;
    while (next < text.size() && is_delimiter(text[next])) {
        ++next;
    }
    rest = text.substr(next);
    return true;
}

bool bad_qname_char(unsigned char c) noexcept {
    return c < 0x21 || c == 0x7F || c == '{' || c == '}';
}

// ---- minimal strict JSON scanner (structure only, no DOM) ------------------
//
// Used for the envelope and INFO, never for payload content (the generated
// codec owns that). Rejects what RFC 8259 rejects; nesting is bounded.

constexpr std::size_t kMaxDepth = 1024;

class Scanner {
public:
    explicit Scanner(std::string_view text) : text_(text) {}

    std::size_t pos() const noexcept { return pos_; }
    bool at_end() const noexcept { return pos_ == text_.size(); }
    const std::string& problem() const noexcept { return problem_; }

    void skip_ws() noexcept {
        while (pos_ < text_.size()) {
            const char c = text_[pos_];
            if (c != ' ' && c != '\t' && c != '\n' && c != '\r') {
                break;
            }
            ++pos_;
        }
    }

    bool consume(char c) noexcept {
        if (pos_ < text_.size() && text_[pos_] == c) {
            ++pos_;
            return true;
        }
        return false;
    }

    bool peek(char c) const noexcept { return pos_ < text_.size() && text_[pos_] == c; }

    bool fail(std::string why) {
        if (problem_.empty()) {
            problem_ = std::move(why) + " at offset " + std::to_string(pos_);
        }
        return false;
    }

    /// A JSON string starting at the current `"`. When `decoded` is non-null
    /// the unescaped UTF-8 is stored there.
    bool string(std::string* decoded) {
        if (!consume('"')) {
            return fail("expected a JSON string");
        }
        while (pos_ < text_.size()) {
            const auto c = static_cast<unsigned char>(text_[pos_]);
            if (c == '"') {
                ++pos_;
                return true;
            }
            if (c < 0x20) {
                return fail("raw control character in a JSON string");
            }
            if (c != '\\') {
                if (decoded != nullptr) {
                    decoded->push_back(static_cast<char>(c));
                }
                ++pos_;
                continue;
            }
            ++pos_;
            if (pos_ == text_.size()) {
                break;
            }
            const char e = text_[pos_++];
            char simple = 0;
            switch (e) {
                case '"': simple = '"'; break;
                case '\\': simple = '\\'; break;
                case '/': simple = '/'; break;
                case 'b': simple = '\b'; break;
                case 'f': simple = '\f'; break;
                case 'n': simple = '\n'; break;
                case 'r': simple = '\r'; break;
                case 't': simple = '\t'; break;
                case 'u': {
                    unsigned point = 0;
                    if (!hex4(point)) {
                        return false;
                    }
                    if (point >= 0xD800 && point <= 0xDBFF) {
                        unsigned low = 0;
                        if (!(consume('\\') && consume('u')) || !hex4(low) || low < 0xDC00 || low > 0xDFFF) {
                            return fail("unpaired surrogate escape");
                        }
                        point = 0x10000 + ((point - 0xD800) << 10) + (low - 0xDC00);
                    } else if (point >= 0xDC00 && point <= 0xDFFF) {
                        return fail("unpaired surrogate escape");
                    }
                    if (decoded != nullptr) {
                        append_utf8(*decoded, point);
                    }
                    continue;
                }
                default: return fail("invalid escape in a JSON string");
            }
            if (decoded != nullptr) {
                decoded->push_back(simple);
            }
        }
        return fail("unterminated JSON string");
    }

    /// Any JSON value, structurally validated, iteratively (bounded depth).
    bool value() {
        std::vector<char> stack;  // '{' or '['
        for (;;) {
            skip_ws();
            // ---- one value ----
            if (peek('{') || peek('[')) {
                const char open = text_[pos_++];
                if (stack.size() >= kMaxDepth) {
                    return fail("JSON nesting too deep");
                }
                stack.push_back(open);
                skip_ws();
                const char close = open == '{' ? '}' : ']';
                if (consume(close)) {
                    stack.pop_back();
                } else {
                    if (open == '{' && !member_key()) {
                        return false;
                    }
                    continue;  // parse the first element / member value
                }
            } else if (peek('"')) {
                if (!string(nullptr)) {
                    return false;
                }
            } else if (!scalar()) {
                return false;
            }
            // ---- after a value: close containers or continue with a comma ----
            for (;;) {
                if (stack.empty()) {
                    return true;
                }
                skip_ws();
                const char close = stack.back() == '{' ? '}' : ']';
                if (consume(close)) {
                    stack.pop_back();
                    continue;
                }
                if (!consume(',')) {
                    return fail("expected ',' or a closing bracket");
                }
                if (stack.back() == '{') {
                    skip_ws();
                    if (!member_key()) {
                        return false;
                    }
                }
                break;  // next value
            }
        }
    }

private:
    bool member_key() {
        if (!string(nullptr)) {
            return false;
        }
        skip_ws();
        if (!consume(':')) {
            return fail("expected ':' after an object member name");
        }
        return true;
    }

    bool hex4(unsigned& out) {
        if (text_.size() - pos_ < 4) {
            return fail("truncated \\u escape");
        }
        out = 0;
        for (int i = 0; i < 4; ++i) {
            const char h = text_[pos_++];
            unsigned digit = 0;
            if (h >= '0' && h <= '9') {
                digit = static_cast<unsigned>(h - '0');
            } else if (h >= 'a' && h <= 'f') {
                digit = static_cast<unsigned>(h - 'a') + 10U;
            } else if (h >= 'A' && h <= 'F') {
                digit = static_cast<unsigned>(h - 'A') + 10U;
            } else {
                return fail("invalid \\u escape");
            }
            out = out * 16U + digit;
        }
        return true;
    }

    static void append_utf8(std::string& out, unsigned point) {
        if (point < 0x80) {
            out.push_back(static_cast<char>(point));
        } else if (point < 0x800) {
            out.push_back(static_cast<char>(0xC0U | (point >> 6)));
            out.push_back(static_cast<char>(0x80U | (point & 0x3FU)));
        } else if (point < 0x10000) {
            out.push_back(static_cast<char>(0xE0U | (point >> 12)));
            out.push_back(static_cast<char>(0x80U | ((point >> 6) & 0x3FU)));
            out.push_back(static_cast<char>(0x80U | (point & 0x3FU)));
        } else {
            out.push_back(static_cast<char>(0xF0U | (point >> 18)));
            out.push_back(static_cast<char>(0x80U | ((point >> 12) & 0x3FU)));
            out.push_back(static_cast<char>(0x80U | ((point >> 6) & 0x3FU)));
            out.push_back(static_cast<char>(0x80U | (point & 0x3FU)));
        }
    }

    bool literal(std::string_view word) {
        if (text_.substr(pos_, word.size()) == word) {
            pos_ += word.size();
            return true;
        }
        return fail("invalid JSON literal");
    }

    bool digits() {
        const std::size_t start = pos_;
        while (pos_ < text_.size() && text_[pos_] >= '0' && text_[pos_] <= '9') {
            ++pos_;
        }
        return pos_ > start;
    }

    bool scalar() {
        if (pos_ == text_.size()) {
            return fail("expected a JSON value");
        }
        const char c = text_[pos_];
        if (c == 't') {
            return literal("true");
        }
        if (c == 'f') {
            return literal("false");
        }
        if (c == 'n') {
            return literal("null");
        }
        if (c == '-' || (c >= '0' && c <= '9')) {
            consume('-');
            if (consume('0')) {
                // no leading zeros
            } else if (!digits()) {
                return fail("invalid JSON number");
            }
            if (consume('.') && !digits()) {
                return fail("invalid JSON number fraction");
            }
            if (peek('e') || peek('E')) {
                ++pos_;
                if (!consume('+')) {
                    consume('-');
                }
                if (!digits()) {
                    return fail("invalid JSON number exponent");
                }
            }
            return true;
        }
        return fail("expected a JSON value");
    }

    std::string_view text_;
    std::size_t pos_ = 0;
    std::string problem_;
};

struct Member {
    std::string name;
    std::string_view value;
};

/// A complete JSON text that is one object; returns its members in order.
bool object_members(std::string_view text, std::vector<Member>& members, std::string& problem) {
    Scanner scan(text);
    scan.skip_ws();
    if (!scan.consume('{')) {
        problem = "not a JSON object";
        return false;
    }
    scan.skip_ws();
    if (!scan.consume('}')) {
        for (;;) {
            scan.skip_ws();
            Member member;
            if (!scan.string(&member.name)) {
                problem = scan.problem();
                return false;
            }
            scan.skip_ws();
            if (!scan.consume(':')) {
                problem = "expected ':' after an object member name";
                return false;
            }
            scan.skip_ws();
            const std::size_t start = scan.pos();
            if (!scan.value()) {
                problem = scan.problem();
                return false;
            }
            member.value = text.substr(start, scan.pos() - start);
            members.push_back(std::move(member));
            scan.skip_ws();
            if (scan.consume('}')) {
                break;
            }
            if (!scan.consume(',')) {
                problem = "expected ',' or '}' in an object";
                return false;
            }
        }
    }
    scan.skip_ws();
    if (!scan.at_end()) {
        problem = "trailing content after the JSON object";
        return false;
    }
    return true;
}

bool decode_string_value(std::string_view text, std::string& out) {
    Scanner scan(text);
    return scan.string(&out) && scan.at_end();
}

constexpr std::array<std::string_view, 8> kErrorCodes = {
    "Unsupported-Version", "Unsupported-Schema", "Unsupported-Service", "Illegal-Operation",
    "Illegal-Argument",    "Illegal-State",      "Internal-Error",      "Invalid-Message"};

}  // namespace

bool is_identifier(std::string_view text) noexcept {
    if (text.empty()) {
        return false;
    }
    for (const char c : text) {
        if (!is_identifier_char(c)) {
            return false;
        }
    }
    return true;
}

std::optional<std::string> check_message_qname(std::string_view message_namespace,
                                               std::string_view local_name) {
    if (local_name.empty()) {
        return std::string("message local name is empty");
    }
    for (const char c : local_name) {
        if (bad_qname_char(static_cast<unsigned char>(c))) {
            return "message local name '" + std::string(local_name) +
                   "' contains whitespace, a control character or a brace";
        }
    }
    for (const char c : message_namespace) {
        if (bad_qname_char(static_cast<unsigned char>(c))) {
            return "message namespace '" + std::string(message_namespace) +
                   "' contains whitespace, a control character or a brace";
        }
    }
    return std::nullopt;
}

std::string global_element_name(std::string_view message_namespace, std::string_view local_name) {
    if (message_namespace == oam_namespace) {
        return std::string(local_name);
    }
    std::string name;
    name.reserve(message_namespace.size() + local_name.size() + 2);
    name.push_back('{');
    name.append(message_namespace);
    name.push_back('}');
    name.append(local_name);
    return name;
}

std::string json_quote(std::string_view text) {
    static constexpr char kHex[] = "0123456789abcdef";
    std::string out;
    out.reserve(text.size() + 2);
    out.push_back('"');
    for (const char ch : text) {
        const auto c = static_cast<unsigned char>(ch);
        switch (c) {
            case '"': out.append("\\\""); break;
            case '\\': out.append("\\\\"); break;
            case '\n': out.append("\\n"); break;
            case '\r': out.append("\\r"); break;
            case '\t': out.append("\\t"); break;
            case '\b': out.append("\\b"); break;
            case '\f': out.append("\\f"); break;
            default:
                if (c < 0x20) {
                    out.append("\\u00");
                    out.push_back(kHex[c >> 4]);
                    out.push_back(kHex[c & 0x0FU]);
                } else {
                    out.push_back(ch);
                }
        }
    }
    out.push_back('"');
    return out;
}

std::string build_init(const std::vector<std::string>& versions, std::string_view schema_version,
                       std::string_view service_id, bool verbose) {
    std::string frame = "INIT {\"versions\":[";
    for (std::size_t i = 0; i < versions.size(); ++i) {
        if (i != 0) {
            frame.push_back(',');
        }
        frame.append(json_quote(versions[i]));
    }
    frame.append("],\"schema\":");
    frame.append(json_quote(schema_version));
    frame.append(",\"verbose\":");
    frame.append(verbose ? "true" : "false");
    frame.append(",\"service_id\":");
    frame.append(json_quote(service_id));
    frame.push_back('}');
    return frame;
}

std::string build_pub(std::string_view topic, std::string_view element_name,
                      std::string_view body_json) {
    std::string frame = "PUB ";
    frame.append(topic);
    frame.append(" {");
    frame.append(json_quote(element_name));
    frame.push_back(':');
    frame.append(body_json);
    frame.push_back('}');
    return frame;
}

std::string build_sub(std::string_view sid, std::string_view element_name, std::string_view topic,
                      std::optional<std::string_view> group) {
    std::string frame = "SUB ";
    frame.append(sid);
    frame.push_back(' ');
    frame.append(element_name);
    frame.push_back(' ');
    frame.append(topic);
    if (group) {
        frame.push_back(' ');
        frame.append(*group);
    }
    return frame;
}

std::string build_unsub(std::string_view sid) {
    std::string frame = "UNSUB ";
    frame.append(sid);
    return frame;
}

bool is_known_error_code(std::string_view code) noexcept {
    for (const std::string_view known : kErrorCodes) {
        if (code == known) {
            return true;
        }
    }
    return false;
}

ServerFrame parse_server_frame(std::string_view frame) {
    ServerFrame out;
    std::string_view keyword;
    std::string_view rest;
    if (!split_token(frame, keyword, rest)) {
        out.problem = "empty frame";
        return out;
    }
    if (keyword == "+OK") {
        if (!rest.empty()) {
            out.problem = "+OK with extra fields";
            return out;
        }
        out.kind = ServerFrame::Kind::ok;
        return out;
    }
    if (keyword == "-ERR") {
        std::string_view code;
        std::string_view details;
        if (!split_token(rest, code, details)) {
            out.problem = "-ERR without an error code";
            return out;
        }
        if (!is_known_error_code(code)) {
            out.problem = "-ERR with unknown error code '" + std::string(code) + "'";
            return out;
        }
        out.kind = ServerFrame::Kind::err;
        out.code = std::string(code);
        out.details = std::string(details);
        return out;
    }
    if (keyword == "INFO") {
        if (rest.empty()) {
            out.problem = "INFO without a payload";
            return out;
        }
        out.kind = ServerFrame::Kind::info;
        out.payload = std::string(rest);
        return out;
    }
    if (keyword == "MSG") {
        std::string_view sid;
        std::string_view payload;
        if (!split_token(rest, sid, payload)) {
            out.problem = "MSG without a subscription id";
            return out;
        }
        if (!is_identifier(sid)) {
            out.problem = "MSG with an invalid subscription id";
            return out;
        }
        if (payload.empty()) {
            out.problem = "MSG without a payload";
            return out;
        }
        out.kind = ServerFrame::Kind::msg;
        out.sid = std::string(sid);
        out.payload = std::string(payload);
        return out;
    }
    out.problem = "unknown server operation '" + std::string(keyword.substr(0, 32)) + "'";
    return out;
}

Unwrapped unwrap_global_element(std::string_view message, std::string_view expected_element) {
    Unwrapped out;
    std::vector<Member> members;
    if (!object_members(message, members, out.problem)) {
        out.problem = "envelope: " + out.problem;
        return out;
    }
    if (members.size() != 1) {
        out.problem = "envelope: expected exactly one global-element member, found " +
                      std::to_string(members.size());
        return out;
    }
    if (members.front().name != expected_element) {
        out.problem = "envelope: global element '" + members.front().name + "' is not the subscribed '" +
                      std::string(expected_element) + "'";
        return out;
    }
    out.ok = true;
    out.body = members.front().value;
    return out;
}

std::optional<std::string> check_info(std::string_view info_json,
                                      const std::vector<std::string>& offered_versions) {
    std::vector<Member> members;
    std::string problem;
    if (!object_members(info_json, members, problem)) {
        return "INFO: " + problem;
    }
    const Member* version = nullptr;
    const Member* server_id = nullptr;
    for (std::size_t i = 0; i < members.size(); ++i) {
        for (std::size_t j = 0; j < i; ++j) {
            if (members[j].name == members[i].name) {
                return "INFO: duplicate member '" + members[i].name + "'";
            }
        }
        if (members[i].name == "version") {
            version = &members[i];
        } else if (members[i].name == "server_id") {
            server_id = &members[i];
        }
    }
    std::string text;
    if (version == nullptr || !decode_string_value(version->value, text)) {
        return std::string("INFO: missing string member 'version'");
    }
    bool offered = false;
    for (const std::string& candidate : offered_versions) {
        offered = offered || candidate == text;
    }
    if (!offered) {
        return "INFO: server selected OWP version '" + text + "', which was not offered";
    }
    std::string id;
    if (server_id == nullptr || !decode_string_value(server_id->value, id)) {
        return std::string("INFO: missing string member 'server_id'");
    }
    return std::nullopt;
}

}  // namespace ams_gra::lacal::owp
