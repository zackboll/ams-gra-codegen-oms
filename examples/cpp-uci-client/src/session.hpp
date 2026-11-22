// CPP-OMS-008: how the application reports runtime outcomes.
//
// Depends only on the runtime's public API (no generated UCI types), so it is
// unit-tested offline. The runtime's ErrorKind / EventKind / OWP code are
// reported unchanged; this file only decides the exit status and whether an
// event ends the session. No recovery (reconnect, resubscribe, replay) is
// attempted for any outcome.
//
// Payload safety: Event::payload (raw MSG text) is NEVER printed. Decoder
// diagnostics are reported by their schema-derived codec path; the generated
// decoder's free-text message is shown only when it contains no quote
// characters, because quoted diagnostics can echo member names or values
// taken from the rejected document.
#pragma once

#include <cstddef>
#include <string>
#include <string_view>

#include "ams_gra/lacal/runtime.hpp"
#include "cli.hpp"

namespace cpp_uci_client {

namespace lacal = ::ams_gra::lacal;

/// Printable ASCII (space allowed), at most `limit` characters, then "...".
inline std::string safe_text(std::string_view text, std::size_t limit) {
    std::string out;
    const std::size_t n = text.size() < limit ? text.size() : limit;
    out.reserve(n + 3);
    for (std::size_t i = 0; i < n; ++i) {
        const auto byte = static_cast<unsigned char>(text[i]);
        out += (byte < 0x20 || byte > 0x7E) ? '?' : text[i];
    }
    if (text.size() > limit) {
        out += "...";
    }
    return out;
}

/// "kind=<ErrorKind>[ owp=<code>][ details=<bounded>] message=<bounded>".
inline std::string describe_error(const lacal::Error& error) {
    std::string text = "kind=";
    text += lacal::to_string(error.kind);
    if (!error.owp_code.empty()) {
        text += " owp=" + safe_text(error.owp_code, 64);
    }
    if (!error.owp_details.empty()) {
        text += " details=\"" + safe_text(error.owp_details, 200) + "\"";
    }
    if (!error.message.empty()) {
        text += " message=\"" + safe_text(error.message, 240) + "\"";
    }
    return text;
}

/// `server_id` of an INFO payload the runtime already validated (object with
/// a string server_id); bounded printable text, or "<unparsed>".
inline std::string info_server_id(std::string_view info) {
    const std::string_view key = "\"server_id\"";
    std::size_t at = info.find(key);
    if (at == std::string_view::npos) {
        return "<unparsed>";
    }
    at = info.find('"', info.find(':', at + key.size()));
    if (at == std::string_view::npos) {
        return "<unparsed>";
    }
    const std::size_t end = info.find('"', at + 1);
    if (end == std::string_view::npos) {
        return "<unparsed>";
    }
    return safe_text(info.substr(at + 1, end - at - 1), 64);
}

/// Exit status for a failed Runtime::connect (TCP, upgrade, owp, INIT, INFO).
inline int connect_exit_status(const lacal::Error& error) {
    switch (error.kind) {
        case lacal::ErrorKind::invalid_argument:
        case lacal::ErrorKind::unsupported_scheme:
            return exit_status::usage;  // refused locally by the runtime; nothing sent
        default:
            return exit_status::connect;
    }
}

/// Exit status for a failed subscribe (no READY was printed).
inline int subscribe_exit_status(const lacal::Error& error) {
    return error.kind == lacal::ErrorKind::broker_rejected ? exit_status::subscribe_rejected : exit_status::session;
}

struct EventOutcome {
    bool fatal = false;  // ends the session (exit 5)
    std::string line;    // one report line; never contains the raw payload
};

/// Classifies one asynchronous runtime event.
inline EventOutcome describe_event(const lacal::Event& event) {
    EventOutcome outcome;
    switch (event.kind) {
        case lacal::EventKind::decode_error: {
            // Client-side rejection: the broker delivered the MSG, the generated
            // decoder (or the runtime's envelope check) refused it. The
            // subscription stays active; reception continues.
            outcome.line = "DECODE-ERROR Log rejected by the client: ";
            outcome.line += lacal::to_string(event.error.kind);
            outcome.line += " path=" + (event.error.codec_path.empty() ? std::string("<envelope>")
                                                                       : safe_text(event.error.codec_path, 160));
            const std::string& reason = event.error.message;
            if (reason.find_first_of("\"'") == std::string::npos) {
                outcome.line += " reason=\"" + safe_text(reason, 160) + "\"";
            } else {
                outcome.line += " reason=<withheld: may quote the rejected document>";
            }
            outcome.line += " payload-bytes=" + std::to_string(event.payload_bytes) + " (not displayed)";
            return outcome;
        }
        case lacal::EventKind::handler_exception:
            outcome.fatal = true;
            outcome.line = "CALLBACK-FAILURE the typed Log callback failed: " + describe_error(event.error);
            return outcome;
        case lacal::EventKind::connection_closed:
            outcome.fatal = true;
            outcome.line = "CONNECTION-LOST " + describe_error(event.error);
            return outcome;
        case lacal::EventKind::server_error:
            outcome.line = "SERVER-ERROR unsolicited -ERR: " + describe_error(event.error);
            return outcome;
        case lacal::EventKind::unknown_subscription:
            outcome.line = "UNKNOWN-SUBSCRIPTION MSG for inactive sid=" + safe_text(event.sid, 32) +
                           " payload-bytes=" + std::to_string(event.payload_bytes) + " (not displayed)";
            return outcome;
        case lacal::EventKind::info:
            outcome.line = "INFO-UPDATE bytes=" + std::to_string(event.payload_bytes);
            return outcome;
        case lacal::EventKind::events_dropped:
            outcome.line = "EVENTS-DROPPED count=" + std::to_string(event.count) + " (runtime event queue full)";
            return outcome;
    }
    outcome.line = "EVENT unrecognized";
    return outcome;
}

}  // namespace cpp_uci_client
