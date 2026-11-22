// CPP-OMS-008: command-line contract of the standalone C++ UCI Log receiver.
//
// Release-agnostic and free of generated UCI types, so it is unit-tested
// offline (examples/cpp-uci-client/tests/unit_test.cpp). Every argument is
// validated here BEFORE any connection is attempted; the C++ LA-CAL runtime
// then re-validates the URL and service ID as the final authority.
#pragma once

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace cpp_uci_client {

/// Process exit statuses (documented in README.md and --help).
namespace exit_status {
constexpr int ok = 0;                  // bound reached, UNSUB acknowledged, runtime closed
constexpr int internal = 1;            // unexpected internal failure (exception escaped)
constexpr int usage = 2;               // invalid arguments; nothing was connected
constexpr int connect = 3;             // TCP/upgrade/owp/INIT/INFO failed; no session
constexpr int subscribe_rejected = 4;  // the broker answered SUB with -ERR
constexpr int session = 5;             // failure after connecting (timeout, loss, overflow, ...)
constexpr int shutdown = 6;            // otherwise clean, but UNSUB or close did not succeed
constexpr int signal_base = 128;       // 128 + SIGINT/SIGTERM after a clean shutdown
}  // namespace exit_status

constexpr unsigned kDefaultDurationSeconds = 30;
constexpr unsigned kMaxDurationSeconds = 3600;
constexpr std::uint32_t kDefaultMaxMessages = 10;
constexpr std::uint32_t kMaxMaxMessages = 100000;
constexpr std::size_t kMaxServiceIdLength = 128;
constexpr std::size_t kMaxUrlLength = 2048;

struct Options {
    std::string url;
    std::string service_id;
    unsigned duration_seconds = kDefaultDurationSeconds;
    std::uint32_t max_messages = kDefaultMaxMessages;
    bool with_rdma = false;
    bool with_survivability = false;
};

struct ParseResult {
    enum class Kind { run, help, error };
    Kind kind = Kind::error;
    Options options;
    std::string error;  // Kind::error only; never echoes control characters
};

/// OWP identifier grammar used by Sleet and the runtime: [A-Za-z0-9_.-]+.
inline bool is_owp_identifier(std::string_view text) noexcept {
    if (text.empty()) {
        return false;
    }
    for (const char c : text) {
        const bool ok = (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') ||
                        c == '_' || c == '-' || c == '.';
        if (!ok) {
            return false;
        }
    }
    return true;
}

namespace detail {

inline std::string lower_ascii(std::string_view text) {
    std::string out(text);
    for (char& c : out) {
        if (c >= 'A' && c <= 'Z') {
            c = static_cast<char>(c - 'A' + 'a');
        }
    }
    return out;
}

/// Printable-ASCII rendering of untrusted argument text (at most 64 chars).
inline std::string printable(std::string_view text) {
    std::string shown(text.substr(0, 64));
    for (char& c : shown) {
        const auto byte = static_cast<unsigned char>(c);
        if (byte < 0x20 || byte >= 0x7F) {
            c = '?';
        }
    }
    return shown;
}

/// Decimal digits only (no sign, no whitespace), within [low, high].
inline std::optional<std::uint32_t> parse_bounded(std::string_view text, std::uint32_t low, std::uint32_t high) {
    if (text.empty() || text.size() > 10) {
        return std::nullopt;
    }
    std::uint64_t value = 0;
    for (const char c : text) {
        if (c < '0' || c > '9') {
            return std::nullopt;
        }
        value = value * 10U + static_cast<std::uint64_t>(c - '0');
    }
    if (value < low || value > high) {
        return std::nullopt;
    }
    return static_cast<std::uint32_t>(value);
}

}  // namespace detail

/// Message explaining why `url` is not acceptable, or nullopt. Native
/// wss:// is refused explicitly: the C++ LA-CAL runtime implements ws://
/// only, and this application neither weakens nor emulates TLS.
inline std::optional<std::string> check_url(std::string_view url) {
    if (url.empty()) {
        return std::string("--url must not be empty");
    }
    if (url.size() > kMaxUrlLength) {
        return std::string("--url is longer than 2048 characters");
    }
    for (const char c : url) {
        const auto byte = static_cast<unsigned char>(c);
        if (byte <= 0x20 || byte >= 0x7F) {
            return std::string("--url contains whitespace, control or non-ASCII characters");
        }
    }
    const std::size_t separator = url.find("://");
    if (separator == std::string_view::npos) {
        return std::string("--url must start with ws://");
    }
    const std::string scheme = detail::lower_ascii(url.substr(0, separator));
    if (scheme == "wss") {
        return std::string(
            "native wss:// (TLS) is not supported: the C++ LA-CAL runtime implements ws:// only and refuses "
            "wss:// rather than weakening TLS; use a ws:// endpoint (see examples/cpp-uci-client/README.md)");
    }
    if (scheme != "ws") {
        return "unsupported URL scheme '" + detail::printable(scheme) + "://'; expected ws://";
    }
    const std::string_view rest = url.substr(separator + 3);
    if (rest.find('#') != std::string_view::npos) {
        return std::string("--url must not contain a fragment ('#')");
    }
    const std::string_view authority = rest.substr(0, rest.find_first_of("/?"));
    if (authority.empty()) {
        return std::string("--url has no host");
    }
    if (authority.find('@') != std::string_view::npos) {
        return std::string("--url must not contain user information ('@')");
    }
    std::string_view host = authority;
    std::string_view port;
    bool has_port = false;
    if (authority.front() == '[') {
        const std::size_t close = authority.find(']');
        if (close == std::string_view::npos) {
            return std::string("--url has an unterminated IPv6 literal");
        }
        host = authority.substr(1, close - 1);
        const std::string_view after = authority.substr(close + 1);
        if (!after.empty()) {
            if (after.front() != ':') {
                return std::string("--url has an invalid authority");
            }
            port = after.substr(1);
            has_port = true;
        }
    } else {
        const std::size_t colon = authority.rfind(':');
        if (colon != std::string_view::npos) {
            host = authority.substr(0, colon);
            port = authority.substr(colon + 1);
            has_port = true;
        }
    }
    if (host.empty()) {
        return std::string("--url has no host");
    }
    if (has_port && !detail::parse_bounded(port, 1, 65535)) {
        return std::string("--url port must be an integer in 1..65535");
    }
    return std::nullopt;
}

/// Parses argv[1..]. `--help` must be the only argument. Value options take
/// exactly one separate value; all options may appear once. Unknown options and the
/// `--opt=value` form are rejected. Nothing here touches the network.
inline ParseResult parse_arguments(const std::vector<std::string_view>& args, bool supports_rdma = false,
                                   bool supports_survivability = false) {
    ParseResult result;
    const auto fail = [&result](std::string message) {
        result.kind = ParseResult::Kind::error;
        result.error = std::move(message);
        return result;
    };
    if (args.size() == 1 && args[0] == "--help") {
        result.kind = ParseResult::Kind::help;
        return result;
    }
    if (args.empty()) {
        return fail("--url and --service-id are required; use --help");
    }
    bool seen_url = false;
    bool seen_service = false;
    bool seen_duration = false;
    bool seen_max = false;
    for (std::size_t i = 0; i < args.size(); ++i) {
        const std::string_view flag = args[i];
        if (flag == "--help") {
            return fail("--help must be the only argument");
        }
        if (flag == "--with-rdma") {
            if (!supports_rdma) {
                return fail("--with-rdma requires a Log + RDMA build (build.sh --with-rdma)");
            }
            if (result.options.with_rdma) {
                return fail("--with-rdma may be given only once");
            }
            result.options.with_rdma = true;
            continue;
        }
        if (flag == "--with-survivability" && supports_survivability) {
            if (result.options.with_survivability) {
                return fail("--with-survivability may be given only once");
            }
            result.options.with_survivability = true;
            continue;
        }
        bool* seen = nullptr;
        if (flag == "--url") {
            seen = &seen_url;
        } else if (flag == "--service-id") {
            seen = &seen_service;
        } else if (flag == "--duration-seconds") {
            seen = &seen_duration;
        } else if (flag == "--max-messages") {
            seen = &seen_max;
        } else {
            return fail("unknown argument '" + detail::printable(flag) + "'; use --help");
        }
        if (*seen) {
            return fail(std::string(flag) + " may be given only once");
        }
        *seen = true;
        if (i + 1 >= args.size() || args[i + 1].empty() || args[i + 1].rfind("--", 0) == 0) {
            return fail(std::string(flag) + " requires a value; use --help");
        }
        const std::string_view value = args[++i];
        if (flag == "--url") {
            result.options.url = std::string(value);
        } else if (flag == "--service-id") {
            result.options.service_id = std::string(value);
        } else if (flag == "--duration-seconds") {
            const auto parsed = detail::parse_bounded(value, 1, kMaxDurationSeconds);
            if (!parsed) {
                return fail("--duration-seconds must be an integer in 1..3600");
            }
            result.options.duration_seconds = *parsed;
        } else {
            const auto parsed = detail::parse_bounded(value, 1, kMaxMaxMessages);
            if (!parsed) {
                return fail("--max-messages must be an integer in 1..100000");
            }
            result.options.max_messages = *parsed;
        }
    }
    if (result.options.with_survivability && !result.options.with_rdma) {
        return fail("--with-survivability requires --with-rdma");
    }
    if (!seen_url) {
        return fail("--url is required; use --help");
    }
    if (!seen_service) {
        return fail("--service-id is required; use --help");
    }
    if (auto problem = check_url(result.options.url)) {
        return fail(std::move(*problem));
    }
    if (result.options.service_id.size() > kMaxServiceIdLength || !is_owp_identifier(result.options.service_id)) {
        return fail("--service-id must be an OWP identifier [A-Za-z0-9_.-], 1..128 characters");
    }
    result.kind = ParseResult::Kind::run;
    return result;
}

/// --help text. `release`/`schema` are the values compiled into this binary.
inline std::string help_text(std::string_view program, std::string_view release, std::string_view schema,
                             std::string_view message, std::string_view topic, bool supports_rdma = false,
                             bool supports_survivability = false) {
    const std::string m(message);
    const std::string t(topic);
    std::string text;
    text += std::string(program) + " --url ws://HOST[:PORT][/PATH] --service-id ID [OPTIONS]\n\n";
    text += "Standalone C++17 UCI Log receiver BUILT FOR UCI " + std::string(release) + " (OWP INIT schema " +
            std::string(schema) + ").\n";
    text += "Subscribes to the global message '" + m + "' on topic '" + t +
            "' through the C++ LA-CAL runtime\nand prints selected fields of each generated programs::oam::LogMT.\n";
    text += "The UCI release is fixed when the executable is built; it cannot be changed at run time.\n\n";
    text += "  --url URL               Sleet endpoint; ws:// only (native wss:// TLS is not supported)\n";
    text += "  --service-id ID         OWP service identity authorized for '" + m + "' on '" + t + "'\n";
    text += "  --duration-seconds N    Receive for at most N seconds, 1..3600 (default 30)\n";
    text += supports_rdma
                ? "  --max-messages N        Stop after N typed messages across enabled subscriptions, 1..100000 (default 10)\n"
                  "  --with-rdma             Also subscribe to 'RDMA_InitializeSetup' on 'uci.rdma' (requires authorization)\n"
                : "  --max-messages N        Stop after N typed Log messages, 1..100000 (default 10)\n";
    if (supports_survivability) {
        text += "  --with-survivability    Also subscribe to 'SurvivabilityRiskLevel' on 'uci.survivability' (requires --with-rdma and authorization)\n";
    }
    text += "  --help                  Print this help without connecting\n\n";
    text += "Exit status:\n";
    text += "  0    a bound was reached; UNSUB acknowledged; runtime closed\n";
    text += "  1    unexpected internal failure\n";
    text += "  2    invalid arguments (nothing was connected)\n";
    text += "  3    connection, WebSocket upgrade, owp negotiation, INIT or INFO failed\n";
    text += "  4    the broker rejected the subscription (-ERR)\n";
    text += "  5    session failure after connecting (SUB timeout, transport loss, queue overflow, callback)\n";
    text += "  6    UNSUB or close did not succeed after an otherwise clean run\n";
    text += "  130  stopped by SIGINT, 143 stopped by SIGTERM (after UNSUB and close)\n";
    return text;
}

}  // namespace cpp_uci_client
