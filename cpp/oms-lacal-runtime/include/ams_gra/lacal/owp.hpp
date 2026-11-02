// CPP-OMS-003: OMS WebSocket Protocol (OWP 1.0) framing and the OMS JSON
// global-element envelope (OMSC-SPC-013 Rev B sections 5 and 6.1.1).
//
// Pure functions only: no I/O, no threads, no allocation beyond the returned
// strings. The runtime (runtime.hpp) is the only production caller; the
// functions are public so the framing can be tested without a network peer.
//
// Grammar (mirrors the spec and the pinned Sleet parser, never extends it):
//   client: INIT <json> | PUB <topic> <message> | SUB <sid> <name> <topic> [group]
//           | UNSUB <sid>
//   server: +OK | -ERR <code> [details] | INFO <json> | MSG <sid> <message>
// Outgoing separators are one space. Incoming separators are runs of SPACE or
// HORIZONTAL TAB. One WebSocket text message carries exactly one operation.
#pragma once

#include <cstddef>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace ams_gra::lacal::owp {

/// The OAM target namespace: global elements in it are named by their bare
/// local name; every other namespace uses Clark notation `{namespace}local`.
inline constexpr std::string_view oam_namespace = "https://www.vdl.afrl.af.mil/programs/oam";

/// The only OWP version this runtime implements.
inline constexpr std::string_view owp_version_1_0 = "1.0";

/// `^[A-Za-z0-9_\-.]+$` -- topics, subscription groups, service IDs and SIDs.
bool is_identifier(std::string_view text) noexcept;

/// Validates a message QName for use on the wire. The local name must be a
/// non-empty run of characters other than whitespace, control characters and
/// braces; the namespace may be empty but must not contain whitespace,
/// control characters or braces (which would make Clark notation ambiguous
/// or split a SUB operation). Returns an explanation, or nullopt when valid.
std::optional<std::string> check_message_qname(std::string_view message_namespace,
                                               std::string_view local_name);

/// The single LA-CAL name formatter: `local` for the OAM namespace (exact
/// match, no normalisation) and `{namespace}local` otherwise. Used for the
/// published JSON member, the expected received member and the SUB name.
std::string global_element_name(std::string_view message_namespace, std::string_view local_name);

/// JSON string literal (with quotes) for UTF-8 `text`, escaping `"`, `\\` and
/// control characters. Precondition: `text` is well-formed UTF-8.
std::string json_quote(std::string_view text);

std::string build_init(const std::vector<std::string>& versions, std::string_view schema_version,
                       std::string_view service_id, bool verbose);
/// `PUB <topic> {"<element>":<body>}`; `body` is the codec's JSON text.
std::string build_pub(std::string_view topic, std::string_view element_name,
                      std::string_view body_json);
std::string build_sub(std::string_view sid, std::string_view element_name, std::string_view topic,
                      std::optional<std::string_view> group);
std::string build_unsub(std::string_view sid);

/// The error codes an OWP 1.0 `-ERR` may carry.
bool is_known_error_code(std::string_view code) noexcept;

struct ServerFrame {
    enum class Kind { ok, err, info, msg, invalid };
    Kind kind = Kind::invalid;
    std::string sid;      // msg
    std::string code;     // err
    std::string details;  // err (may be empty)
    std::string payload;  // info / msg (JSON text, unparsed)
    std::string problem;  // invalid: why the frame is not OWP 1.0
};

ServerFrame parse_server_frame(std::string_view frame);

/// Result of removing the global-element envelope from a received message.
struct Unwrapped {
    bool ok = false;
    std::string_view body;  // view into the input: the member's value text
    std::string problem;    // when !ok
};

/// Requires `message` to be exactly one JSON object with exactly one member
/// whose (unescaped) name equals `expected_element`, and returns the exact
/// source text of that member's value for the generated codec to decode.
/// The value's JSON syntax is checked structurally here; its content is the
/// codec's responsibility.
Unwrapped unwrap_global_element(std::string_view message, std::string_view expected_element);

/// Checks a server INFO payload: a JSON object with distinct members, a string
/// `version` that is one of `offered_versions`, and a string `server_id`.
std::optional<std::string> check_info(std::string_view info_json,
                                      const std::vector<std::string>& offered_versions);

}  // namespace ams_gra::lacal::owp
