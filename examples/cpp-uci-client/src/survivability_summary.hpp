// Bounded display subset of the generated SurvivabilityRiskLevelMT record.
#pragma once

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <string>
#include <string_view>

#include "log_summary.hpp"
#include "oam.hpp"

namespace cpp_uci_client {

struct SurvivabilitySummary {
    // Independent application display bounds, not validation rules.
    static constexpr std::size_t settings_limit = 3;
    static constexpr std::size_t levels_limit = 4;
    struct Setting {
        ::programs::oam::RiskPrioritizationEnum priority{};
        std::size_t level_count = 0;
        std::array<::programs::oam::VulnerabilityTypeEnum, levels_limit> levels{};
    };
    ::programs::oam::MessageModeEnum mode{};
    ::programs::oam::ClassificationEnum classification{};
    std::string risk_level_uuid;
    std::string schema_version;
    std::string header_timestamp;
    std::size_t setting_count = 0;
    std::array<Setting, settings_limit> settings{};
};

inline SurvivabilitySummary summarize_survivability(const ::programs::oam::SurvivabilityRiskLevelMT& value) {
    SurvivabilitySummary summary;
    summary.mode = value.messageheader.mode;
    summary.classification = value.securityinformation.classification;
    summary.risk_level_uuid = value.messagedata.survivabilityrisklevelid.uuid.value();
    summary.schema_version = value.messageheader.schemaversion.value();
    summary.header_timestamp = value.messageheader.timestamp.value().substr(0, timestamp_display_limit + 1);
    const auto& settings = value.messagedata.risksetting.values();
    summary.setting_count = settings.size();
    for (std::size_t i = 0; i < std::min(settings.size(), summary.settings.size()); ++i) {
        const auto& levels = settings[i].risklevel.values();
        auto& setting = summary.settings[i];
        setting.priority = settings[i].riskprioritization;
        setting.level_count = levels.size();
        for (std::size_t j = 0; j < std::min(levels.size(), setting.levels.size()); ++j) {
            setting.levels[j] = levels[j].allowedrisk;
        }
    }
    return summary;
}

inline const char* mode_name(::programs::oam::MessageModeEnum value) noexcept {
    using E = ::programs::oam::MessageModeEnum;
    switch (value) {
        case E::LIVE: return "LIVE";
        case E::EXERCISE: return "EXERCISE";
        case E::SIMULATION: return "SIMULATION";
        case E::NONEXERCISESIMULATION: return "NONEXERCISE_SIMULATION";
    }
    return "UNRECOGNIZED";
}

inline const char* priority_name(::programs::oam::RiskPrioritizationEnum value) noexcept {
    using E = ::programs::oam::RiskPrioritizationEnum;
    switch (value) {
        case E::HIGH: return "HIGH";
        case E::MEDIUM: return "MEDIUM";
        case E::LOW: return "LOW";
        case E::NONE: return "NONE";
    }
    return "UNRECOGNIZED";
}

inline const char* allowed_risk_name(::programs::oam::VulnerabilityTypeEnum value) noexcept {
    using E = ::programs::oam::VulnerabilityTypeEnum;
    switch (value) {
        case E::NONE: return "NONE";
        case E::ACQUISITION: return "ACQUISITION";
        case E::TRACK: return "TRACK";
        case E::LAUNCH: return "LAUNCH";
        case E::INTERCEPT: return "INTERCEPT";
        case E::KILL: return "KILL";
        case E::ALL: return "ALL";
    }
    return "UNRECOGNIZED";
}

inline std::string format_summary(std::uint64_t sequence, const SurvivabilitySummary& s, std::string_view built_schema) {
    std::string line = "RECEIVED SurvivabilityRiskLevel #" + std::to_string(sequence);
    line += " risk-level=" + bounded_text(s.risk_level_uuid, 36);
    line += " mode=";
    line += mode_name(s.mode);
    line += " classification=";
    line += classification_name(s.classification);
    line += " schema=" + bounded_text(s.schema_version, 57);
    line += " header-timestamp=" + bounded_text(s.header_timestamp, timestamp_display_limit);
    line += " risk-settings=" + std::to_string(s.setting_count);
    const auto shown = std::min(s.setting_count, s.settings.size());
    for (std::size_t i = 0; i < shown; ++i) {
        const auto& setting = s.settings[i];
        line += " ";
        line += priority_name(setting.priority);
        line += ":";
        const auto levels = std::min(setting.level_count, setting.levels.size());
        for (std::size_t j = 0; j < levels; ++j) {
            if (j != 0) {
                line += ",";
            }
            line += allowed_risk_name(setting.levels[j]);
        }
        if (levels < setting.level_count) {
            line += ",+" + std::to_string(setting.level_count - levels);
        }
    }
    if (shown < s.setting_count) {
        line += " settings-omitted=" + std::to_string(s.setting_count - shown);
    }
    if (s.schema_version != built_schema) {
        line += " schema-differs-from-build=" + std::string(built_schema);
    }
    return line;
}

}  // namespace cpp_uci_client