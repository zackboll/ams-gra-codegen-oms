//! Carrier-private scanner: no generated top-level parser/helper identifier.
pub(super) const CARRIER: &str = r#"class {name} {
public:
    static std::optional<{name}> create(std::string_view value) {
        std::string lexical;
        bool pending = false;
        for (char c : value) {
            if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
                pending = !lexical.empty();
            } else {
                if (pending) { lexical.push_back(' '); pending = false; }
                lexical.push_back(c);
            }
        }
        if (!valid(lexical)) return std::nullopt;
        return {name}(std::move(lexical));
    }
    // Explicit copy operations suppress destructive move, preserving sources.
    {name}(const {name}&) = default;
    {name}& operator=(const {name}&) = default;
    const std::string& value() const noexcept { return value_; }
private:
    explicit {name}(std::string lexical) : value_(std::move(lexical)) {}
    static bool digit(char c) { return c >= '0' && c <= '9'; }
    static bool two(std::string_view text, unsigned& value) {
        if (!digit(text[0]) || !digit(text[1])) return false;
        value = static_cast<unsigned>(text[0] - '0') * 10 + static_cast<unsigned>(text[1] - '0');
        return true;
    }
    static bool valid(std::string_view text) {
        if (text.size() < 9 || text.back() != 'Z') return false;
        text.remove_suffix(1);
        if (text[2] != ':' || text[5] != ':') return false;
        unsigned hour = 0, minute = 0, second = 0;
        if (!two(text.substr(0, 2), hour) || !two(text.substr(3, 2), minute)
            || !two(text.substr(6, 2), second)) return false;
        // XSD 1.0 Appendix D includes second 60 with arbitrary fractions.
        if (hour > 24 || minute > 59 || second > 60) return false;
        const auto fraction = text.substr(8);
        bool zero = true;
        if (!fraction.empty()) {
            if (fraction.size() < 2 || fraction[0] != '.') return false;
            for (std::size_t i = 1; i < fraction.size(); ++i) {
                if (!digit(fraction[i])) return false;
                if (fraction[i] != '0') zero = false;
            }
        }
        return hour != 24 || (minute == 0 && second == 0 && zero);
    }
    std::string value_;
};
"#;
