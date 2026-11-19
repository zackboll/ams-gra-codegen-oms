// CPP-OMS-008: the bounded, schema-checked subset of a generated
// programs::oam::LogMT that the application displays.
//
// summarize() runs in the typed callback (runtime dispatcher thread) and
// copies ONLY these few validated fields; the full payload (Label, Details,
// Data, security caveats, ...) is never copied, queued or printed. Every
// field is taken from the generated model, whose decoder has already
// validated it against the authoritative UCI facets.
#pragma once

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>

#include "oam.hpp"

namespace cpp_uci_client {

struct LogSummary {
    ::programs::oam::LogSeverityEnum severity{};
    ::programs::oam::ClassificationEnum classification{};
    std::string header_timestamp;                    // MessageHeader.Timestamp (DateTimeType)
    std::string log_timestamp;                       // MessageData.Timestamp (DateTimeType)
    std::string schema_version;                      // MessageHeader.SchemaVersion
    std::string system_uuid;                         // MessageHeader.SystemID.UUID
    std::optional<std::string> header_service_uuid;  // MessageHeader.ServiceID.UUID
    std::optional<std::string> service_uuid;         // MessageData.ServiceID.UUID
    std::optional<std::string> subsystem_uuid;       // MessageData.SubsystemID.UUID
};

/// Exhaustive over the generated enumerators: a generated enum that gains or
/// loses a value fails the strict (-Wall -Werror => -Wswitch) build.
inline const char* severity_name(::programs::oam::LogSeverityEnum value) noexcept {
    using E = ::programs::oam::LogSeverityEnum;
    switch (value) {
        case E::TRACE: return "TRACE";
        case E::DEBUG: return "DEBUG";
        case E::INFO: return "INFO";
        case E::WARNING: return "WARNING";
        case E::ERROR: return "ERROR";
        case E::FATAL: return "FATAL";
    }
    return "UNRECOGNIZED";  // unreachable for decoder-produced values
}

inline const char* classification_name(::programs::oam::ClassificationEnum value) noexcept {
    using E = ::programs::oam::ClassificationEnum;
    switch (value) {
        case E::R: return "R";
        case E::C: return "C";
        case E::S: return "S";
        case E::TS: return "TS";
        case E::U: return "U";
    }
    return "UNRECOGNIZED";
}

/// Typed extraction; called from the runtime's dispatcher thread.
inline LogSummary summarize(const ::programs::oam::LogMT& log) {
    LogSummary summary;
    summary.severity = log.messagedata.severity;
    summary.classification = log.securityinformation.classification;
    summary.header_timestamp = log.messageheader.timestamp.value();
    summary.log_timestamp = log.messagedata.timestamp.value();
    summary.schema_version = log.messageheader.schemaversion.value();
    summary.system_uuid = log.messageheader.systemid.uuid.value();
    if (log.messageheader.serviceid) {
        summary.header_service_uuid = log.messageheader.serviceid->uuid.value();
    }
    if (log.messagedata.serviceid) {
        summary.service_uuid = log.messagedata.serviceid->uuid.value();
    }
    if (log.messagedata.subsystemid) {
        summary.subsystem_uuid = log.messagedata.subsystemid->uuid.value();
    }
    return summary;
}

/// Printable ASCII, at most `limit` characters (then "..."). The generated
/// types already restrict these fields; this is defence in depth for output.
inline std::string bounded_text(std::string_view text, std::size_t limit) {
    std::string out;
    const std::size_t n = text.size() < limit ? text.size() : limit;
    out.reserve(n + 3);
    for (std::size_t i = 0; i < n; ++i) {
        const auto byte = static_cast<unsigned char>(text[i]);
        out += (byte < 0x21 || byte > 0x7E) ? '?' : text[i];
    }
    if (text.size() > limit) {
        out += "...";
    }
    return out;
}

/// One display line. `built_schema` is the release compiled into the binary.
inline std::string format_summary(std::uint64_t sequence, const LogSummary& s, std::string_view built_schema) {
    std::string line = "RECEIVED Log #" + std::to_string(sequence);
    line += " severity=";
    line += severity_name(s.severity);
    line += " classification=";
    line += classification_name(s.classification);
    line += " schema=" + bounded_text(s.schema_version, 57);
    line += " header-timestamp=" + bounded_text(s.header_timestamp, 64);
    line += " log-timestamp=" + bounded_text(s.log_timestamp, 64);
    line += " system=" + bounded_text(s.system_uuid, 36);
    if (s.header_service_uuid) {
        line += " header-service=" + bounded_text(*s.header_service_uuid, 36);
    }
    if (s.service_uuid) {
        line += " service=" + bounded_text(*s.service_uuid, 36);
    }
    if (s.subsystem_uuid) {
        line += " subsystem=" + bounded_text(*s.subsystem_uuid, 36);
    }
    if (s.schema_version != built_schema) {
        line += " schema-differs-from-build=" + std::string(built_schema);
    }
    return line;
}

}  // namespace cpp_uci_client
