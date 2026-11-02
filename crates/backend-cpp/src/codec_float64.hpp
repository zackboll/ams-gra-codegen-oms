// ---- Float64 (CPP-OMS-004) -------------------------------------------------
//
// Emitted only when the selected closure contains a Float64 value; a
// Float64-free codec is byte-identical to CPP-OMS-002.
//
// OMSC-SPC-013 Rev B 6.1.4 maps `xs:double` to a JSON number or to exactly one
// of the strings "NaN", "Infinity", "-Infinity".
//
// Decode: a finite JSON number lexeme (already restricted to the RFC 8259
// grammar by the reader) is converted ONCE by `std::from_chars(...,
// std::chars_format::general)`, which C++17 specifies as correctly rounded
// (round-to-nearest, ties-to-even) and locale-independent. A lexeme whose
// correctly rounded value is +/-infinity is rejected (decimal overflow is not
// a special value); one whose correctly rounded value is zero becomes a zero of
// the lexeme's sign (`from_chars` reports both as `result_out_of_range` and
// leaves the output untouched, so the codec tells them apart from the decimal
// exponent alone). Signed zero, subnormals and every finite binary64 are kept
// bit for bit. Quoted numbers, other special spellings ("nan", "inf",
// "+Infinity", ...), booleans, null, arrays and objects are rejected.
//
// Encode: NaN (any payload or sign) -> "NaN", +inf -> "Infinity", -inf ->
// "-Infinity". A finite value is written with the SHORTEST decimal digit
// string that converts back to the same binary64 (the closest such string if
// several exist), obtained from `std::to_chars(..., chars_format::scientific)`
// without a precision, and laid out by the ECMAScript Number::toString rule
// (ECMA-262 6.1.6.1.20) except that a negative zero keeps its sign ("-0").
// Moving decimal digits around a decimal point never changes the value.
//
// Toolchain: requires the C++17 floating-point <charconv> (`__cpp_lib_to_chars
// >= 201611L`). The `result_out_of_range` behavior relied on above (only an
// infinite or zero correctly rounded result is reported, subnormal results are
// returned) is the qualified libstdc++ >= 12 implementation; any other
// standard library is refused at compile time unless the integrator defines
// SERVICE_CODEC_ASSUME_CONFORMING_CHARCONV after qualifying it.
#if !defined(__cpp_lib_to_chars) || __cpp_lib_to_chars < 201611L
#error "OMS Float64 codec requires C++17 floating-point std::from_chars/std::to_chars (__cpp_lib_to_chars >= 201611L)"
#endif
#if !defined(SERVICE_CODEC_ASSUME_CONFORMING_CHARCONV)
#if !defined(__GLIBCXX__) || !defined(_GLIBCXX_RELEASE) || _GLIBCXX_RELEASE < 12
#error "OMS Float64 codec is qualified only with libstdc++ >= 12 <charconv>; define SERVICE_CODEC_ASSUME_CONFORMING_CHARCONV after qualifying another library"
#endif
#endif
static_assert(std::numeric_limits<double>::is_iec559 && std::numeric_limits<double>::radix == 2 &&
                  std::numeric_limits<double>::digits == 53,
              "OMS Float64 requires IEEE binary64 double");

/// True when a JSON number lexeme's decimal magnitude is below one (its first
/// significant digit lies right of the decimal point after applying the
/// exponent). Linear in the lexeme; the exponent saturates, so arbitrarily
/// long digit or exponent strings are handled in bounded arithmetic.
inline bool lexeme_magnitude_below_one(std::string_view lexeme) {
    std::size_t at = (!lexeme.empty() && lexeme[0] == '-') ? 1 : 0;
    long long adjusted = 0;  // decimal exponent of the first significant digit
    bool found = false;
    long long integer_digits = 0;
    while (at < lexeme.size() && lexeme[at] >= '0' && lexeme[at] <= '9') {
        if (!found && lexeme[at] != '0') {
            found = true;
            integer_digits = 0;
        }
        if (found && integer_digits < (1LL << 50)) {
            ++integer_digits;
        }
        ++at;
    }
    if (found) {
        adjusted = integer_digits - 1;
    }
    if (at < lexeme.size() && lexeme[at] == '.') {
        ++at;
        long long position = 0;
        while (at < lexeme.size() && lexeme[at] >= '0' && lexeme[at] <= '9') {
            if (position < (1LL << 50)) {
                ++position;
            }
            if (!found && lexeme[at] != '0') {
                found = true;
                adjusted = -position;
            }
            ++at;
        }
    }
    if (!found) {
        return true;  // every digit is zero
    }
    if (at < lexeme.size() && (lexeme[at] == 'e' || lexeme[at] == 'E')) {
        ++at;
        bool negative = false;
        if (at < lexeme.size() && (lexeme[at] == '+' || lexeme[at] == '-')) {
            negative = lexeme[at] == '-';
            ++at;
        }
        long long exponent = 0;
        while (at < lexeme.size() && lexeme[at] >= '0' && lexeme[at] <= '9') {
            if (exponent < (1LL << 52)) {
                exponent = exponent * 10 + (lexeme[at] - '0');
            }
            ++at;
        }
        adjusted += negative ? -exponent : exponent;
    }
    return adjusted < 0;
}

/// Decode one Float64 JSON value (number, or one of the three OMS strings).
inline std::optional<double> get_f64(const Value& value, CodecError& error, const Path* path) {
    if (value.kind == Value::Kind::String) {
        if (value.text == "NaN") {
            return std::numeric_limits<double>::quiet_NaN();
        }
        if (value.text == "Infinity") {
            return std::numeric_limits<double>::infinity();
        }
        if (value.text == "-Infinity") {
            return -std::numeric_limits<double>::infinity();
        }
        return fail(error, path,
                    "Float64 string must be exactly \"NaN\", \"Infinity\" or \"-Infinity\"");
    }
    if (value.kind != Value::Kind::Number) {
        return fail(error, path,
                    std::string("expected a Float64 number or \"NaN\"/\"Infinity\"/\"-Infinity\", found ") +
                        kind_name(value.kind));
    }
    const char* const first = value.text.data();
    const char* const last = first + value.text.size();
    double result = 0.0;
    const std::from_chars_result parsed = std::from_chars(first, last, result, std::chars_format::general);
    if (parsed.ptr != last) {
        return fail(error, path, "number is not a complete binary64 lexeme");
    }
    if (parsed.ec == std::errc()) {
        return result;
    }
    if (parsed.ec == std::errc::result_out_of_range) {
        if (lexeme_magnitude_below_one(value.text)) {
            // Correctly rounded to zero: keep the lexeme's sign.
            return value.text[0] == '-' ? -0.0 : 0.0;
        }
        return fail(error, path, "number overflows binary64 (magnitude exceeds the largest finite double)");
    }
    return fail(error, path, "number is not a valid binary64 lexeme");
}

/// A checked named Float64 carrier (`Carrier::create(double)`): the generated
/// model alone decides domain membership.
template <typename Carrier>
std::optional<Carrier> get_checked_f64(const Value& value, CodecError& error, const Path* path) {
    const std::optional<double> number = get_f64(value, error, path);
    if (!number) {
        return std::nullopt;
    }
    std::optional<Carrier> checked = Carrier::create(*number);
    if (!checked) {
        return fail(error, path, "value rejected by the generated model's domain check");
    }
    return checked;
}

/// Write one Float64 (see the layout rule at the top of this section).
inline bool put_f64(std::string& out, double value, CodecError& error, const Path* path) {
    if (std::isnan(value)) {
        out += "\"NaN\"";
        return true;
    }
    if (std::isinf(value)) {
        out += value > 0.0 ? "\"Infinity\"" : "\"-Infinity\"";
        return true;
    }
    // Longest shortest-scientific form is 24 characters
    // ("-2.2250738585072014e-308").
    char buffer[40];
    const std::to_chars_result written =
        std::to_chars(buffer, buffer + sizeof buffer, value, std::chars_format::scientific);
    if (written.ec != std::errc()) {
        return fail_bool(error, path, "Float64 could not be formatted");
    }
    const std::string_view text(buffer, static_cast<std::size_t>(written.ptr - buffer));
    std::size_t at = 0;
    const bool negative = text[0] == '-';
    if (negative) {
        at = 1;
    }
    std::string digits;
    while (at < text.size() && text[at] != 'e') {
        if (text[at] != '.') {
            digits += text[at];
        }
        ++at;
    }
    ++at;  // 'e'
    const bool exponent_negative = text[at] == '-';
    ++at;  // sign (always present)
    int exponent = 0;
    for (; at < text.size(); ++at) {
        exponent = exponent * 10 + (text[at] - '0');
    }
    if (exponent_negative) {
        exponent = -exponent;
    }
    // ECMA-262 Number::toString: k significant digits, point position n.
    const int k = static_cast<int>(digits.size());
    const int n = exponent + 1;
    if (negative) {
        out += '-';
    }
    if (k <= n && n <= 21) {
        out += digits;
        out.append(static_cast<std::size_t>(n - k), '0');
    } else if (0 < n && n <= 21) {
        out.append(digits, 0, static_cast<std::size_t>(n));
        out += '.';
        out.append(digits, static_cast<std::size_t>(n), std::string::npos);
    } else if (-6 < n && n <= 0) {
        out += "0.";
        out.append(static_cast<std::size_t>(-n), '0');
        out += digits;
    } else {
        out += digits[0];
        if (k > 1) {
            out += '.';
            out.append(digits, 1, std::string::npos);
        }
        out += n - 1 < 0 ? "e-" : "e+";
        out += std::to_string(n - 1 < 0 ? 1 - n : n - 1);
    }
    return true;
}

/// Encode a checked named Float64 carrier, re-validating the stored value
/// through the model's own `create` (a carrier whose bytes were forged cannot
/// leave the process).
template <typename Carrier>
bool put_checked_f64(std::string& out, const Carrier& value, CodecError& error, const Path* path) {
    if (!Carrier::create(value.value())) {
        return fail_bool(error, path, "value rejected by the generated model's domain check");
    }
    return put_f64(out, value.value(), error, path);
}

