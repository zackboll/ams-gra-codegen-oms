//! Standalone exact-profile renderer, with private factored scanner pieces.
pub(super) fn render(name: &str) -> String {
    TEMPLATE.replace("{name}", name)
}

const TEMPLATE: &str = r#"class {name} {
public:
    static std::optional<{name}> create(std::string_view text) {
        if (!is_valid(text)) return std::nullopt;
        return {name}(std::string(text));
    }
    {name}(const {name}&) = default;
    {name}& operator=(const {name}&) = default;
    const std::string& value() const noexcept { return value_; }
private:
    explicit {name}(std::string text) : value_(std::move(text)) {}
    static bool hex_byte(unsigned char b) {
        return (b >= '0' && b <= '9') || (b >= 'a' && b <= 'f') || (b >= 'A' && b <= 'F');
    }
    static bool hex_group(std::string_view text) {
        if (text.size() > 4) return false;
        for (const unsigned char b : text) if (!hex_byte(b)) return false;
        return true;
    }
    static std::optional<std::size_t> hex_colon(std::string_view text, std::size_t start) {
        std::size_t end = start;
        while (end < text.size() && hex_byte(static_cast<unsigned char>(text[end]))) ++end;
        if (end - start <= 4 && end < text.size() && text[end] == ':') return end + 1;
        return std::nullopt;
    }
    static bool ipv4_octet(std::string_view text) {
        if (text.empty() || text.size() > 3) return false;
        unsigned value = 0;
        for (const unsigned char b : text) {
            if (b < '0' || b > '9') return false;
            value = value * 10 + (b - '0');
        }
        return value <= 255;
    }
    static bool embedded_ipv4(std::string_view text) {
        std::size_t start = 0, count = 0;
        for (std::size_t end = 0; end <= text.size(); ++end) {
            if (end != text.size() && text[end] != '.') continue;
            if (!ipv4_octet(text.substr(start, end - start))) return false;
            ++count;
            if (end == text.size()) break;
            start = end + 1;
        }
        return count == 4;
    }
    static bool suffix(std::string_view text) {
        if (hex_group(text) || text == ":") return true;
        if (const auto end = hex_colon(text, 0)) {
            const auto tail = text.substr(*end);
            if (hex_group(tail) || tail == ":") return true;
        }
        return embedded_ipv4(text);
    }
    static bool after_prefix(std::string_view text, std::size_t start) {
        for (unsigned count = 0; count <= 5; ++count) {
            if (suffix(text.substr(start))) return true;
            if (count == 5) break;
            const auto end = hex_colon(text, start);
            if (!end) break;
            start = *end;
        }
        return false;
    }
    static bool is_valid(std::string_view text) {
        if (text.size() < 2 || text.size() > 45) return false;
        if (const auto end = hex_colon(text, 0)) {
            if (after_prefix(text, *end)) return true;
        }
        return text.substr(0, 2) == "::" && after_prefix(text, 2);
    }
    std::string value_;
};
"#;

#[cfg(test)]
#[path = "../../../tests/task062_probes.rs"]
mod probes;

#[cfg(test)]
mod tests {
    #[test]
    fn standalone_ipv6_compiler_corpus_and_lifecycle() {
        super::probes::run("cpp", &super::render("Address"));
    }
}
