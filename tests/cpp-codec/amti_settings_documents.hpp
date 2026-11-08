// CPP-OMS-005: independently authored authentic UCI AMTI_SettingsCommand
// documents and their typed expectations, shared by
//   tests/cpp-codec/amti_settings_consumer.cpp    (codec only)
//   tests/cpp-lacal/real_amti_settings_sleet.cpp  (pinned Sleet)
//
// Every document is written by hand in canonical form (schema order, no
// whitespace). UCI 2.5 and 2.6 differ for this message and are NOT adapted
// from one another:
//   * 2.5: MessageOutput is AMTI_MessageOutputsEnum 0..4, declared in
//     AMTI_SettingsCommandMDT after EmissionBalanceState;
//   * 2.6: MessageOutput is CapabilityMessageOutputsEnum 0..49, declared in
//     CapabilitySettingsCommandBaseType after RepresentationKey, so it moves
//     before ElectronicProtectionOptions on the wire;
//   * SchemaVersion is 002.5.0 / 002.6.0.
// UnassignAll (EmptyType, `[a-zA-Z]{0}`, length 0, optional) is identical in
// both releases.
//
// Requires UCI_AMTI_VERSION=25|26 and the generated `oam.hpp` of that release.
#pragma once

#include <cstdlib>
#include <iostream>
#include <string>
#include <type_traits>
#include <variant>
#include <vector>

#include "oam.hpp"

#ifndef UCI_AMTI_VERSION
#error "define UCI_AMTI_VERSION=25 or 26"
#endif

namespace amti {

namespace m = ::programs::oam;

#if UCI_AMTI_VERSION == 25
inline const std::string kVersion = "002.5.0";
inline constexpr std::size_t kMessageOutputMax = 4;
#elif UCI_AMTI_VERSION == 26
inline const std::string kVersion = "002.6.0";
inline constexpr std::size_t kMessageOutputMax = 49;
#else
#error "UCI_AMTI_VERSION must be 25 or 26"
#endif

/// The release's MessageOutput enumeration (AMTI_MessageOutputsEnum in 2.5,
/// CapabilityMessageOutputsEnum in 2.6); both have AMTI_ACTIVITY and ENTITY.
using MessageOutput = std::decay_t<decltype(std::declval<m::AMTISettingsCommandMDT>().messageoutput.values()[0])>;

inline const std::string kSystem = "550e8400-e29b-41d4-a716-446655440000";
inline const std::string kCommand = "11111111-2222-4333-8444-555555555555";
inline const std::string kSubsystem = "66666666-7777-4888-9999-aaaaaaaaaaaa";
inline const std::string kMdf = "0a0b0c0d-0e0f-4a1b-8c2d-3e4f5a6b7c8d";
inline const std::string kSubCategory = "12345678-9abc-4def-8123-456789abcdef";
inline const std::string kFile = "fedcba98-7654-4321-8fed-cba987654321";
inline const std::string kCapability = "00000000-0000-4000-8000-000000000001";
inline const std::string kStamp = "2026-01-01T00:00:00Z";
inline const std::string kRoot = "AMTI_SettingsCommandMT";
inline const std::string kUnassignPath = "AMTI_SettingsCommandMT.MessageData.UnassignAll";

/// `text` with the single occurrence of `from` replaced by `to` (test setup
/// fails loudly if `from` is absent or ambiguous).
inline std::string with(const std::string& text, const std::string& from, const std::string& to) {
    const std::size_t at = text.find(from);
    if (at == std::string::npos || text.find(from, at + 1) != std::string::npos) {
        std::cerr << "test setup: '" << from << "' must occur exactly once\n";
        std::exit(2);
    }
    std::string result = text;
    result.replace(at, from.size(), to);
    return result;
}

inline const std::string kSecurity =
    R"("SecurityInformation":{"Classification":"U","OwnerProducer":[{"GovernmentIdentifier":"USA"}]})";
inline const std::string kHeader = R"("MessageHeader":{"SystemID":{"UUID":")" + kSystem +
                                   R"("},"Timestamp":")" + kStamp + R"(","SchemaVersion":")" + kVersion +
                                   R"(","Mode":"LIVE"})";

/// The whole payload body around one MessageData object text.
inline std::string body(const std::string& data) {
    return "{" + kSecurity + "," + kHeader + ",\"MessageData\":" + data + "}";
}

// ---- minimal: the three required MessageData members ---------------------
inline const std::string kRequired = R"("CommandID":{"UUID":")" + kCommand + R"("},"CommandState":"NEW","SubsystemID":{"UUID":")" +
                                     kSubsystem + R"("})";
/// UnassignAll absent.
inline const std::string kAbsent = body("{" + kRequired + "}");
/// UnassignAll present as the zero-length EmptyType value.
inline const std::string kPresent = body("{" + kRequired + R"(,"UnassignAll":""})");

// ---- full: every MessageData member of the closure, release order -----------
inline const std::string kOutputs = R"("MessageOutput":["AMTI_ACTIVITY","ENTITY"])";
inline const std::string kEp =
    R"("ElectronicProtectionOptions":[{"CapabilityID":{"UUID":")" + kCapability +
    R"("},"ElectronicProtectionOptions":[{"ElectronicProtectionOptionsKey":{"Key":"EP-7","SystemName":"Jammer A"},"ElectronicProtectionOptionStatus":"DISABLE"}]}])";
inline const std::string kFullHead =
    R"({"CommandID":{"UUID":")" + kCommand + R"(","DescriptiveLabel":"AMTI settings #1"},"CommandState":"UPDATE",)"
    R"("SubsystemID":{"UUID":")" + kSubsystem + R"("},)"
    R"("ActivateMDF":[{"MDF_ID":{"UUID":")" + kMdf + R"("},"SubCategory":[{"SubCategoryID":{"UUID":")" + kSubCategory +
    R"("},"FileID":{"UUID":")" + kFile + R"("}}]}],)"
    R"("CapabilityRank":[{"CapabilityID":{"UUID":")" + kCapability + R"("},"CapabilityRank":65535}],)"
    R"("CapabilityState":[{"CapabilityID":{"UUID":")" + kCapability + R"("},"CapabilityCommandState":"RESET"}],)"
    R"("RepresentationKey":{"Key":"REP-1","SystemName":"Sys 1"})";
#if UCI_AMTI_VERSION == 25
inline const std::string kFullData = kFullHead + "," + kEp + R"(,"EmissionBalanceState":"BALANCED",)" + kOutputs +
                                     R"(,"UnassignAll":"","DefaultEmcon":{"EmconLevel":"EMCON_LEVEL_3"}})";
#else
inline const std::string kFullData = kFullHead + "," + kOutputs + "," + kEp +
                                     R"(,"EmissionBalanceState":"BALANCED","UnassignAll":"","DefaultEmcon":{"EmconLevel":"EMCON_LEVEL_3"}})";
#endif
inline const std::string kFull = body(kFullData);

// ---- typed expectations ------------------------------------------------------

inline void expect(std::vector<std::string>& failures, bool condition, const char* what) {
    if (!condition) {
        failures.emplace_back(what);
    }
}

inline void verify_envelope(const m::AMTISettingsCommandMT& v, std::vector<std::string>& f) {
    expect(f, v.securityinformation.classification == m::ClassificationEnum::U, "Classification U");
    const auto& owners = v.securityinformation.ownerproducer.values();
    expect(f, owners.size() == 1, "one OwnerProducer");
    if (!owners.empty()) {
        const auto* government = std::get_if<m::OwnerProducerChoiceType::GovernmentIdentifier>(&owners[0].value);
        expect(f, government != nullptr && government->value == m::OwnerProducerEnum::USA, "OwnerProducer USA");
    }
    expect(f, v.messageheader.systemid.uuid.value() == kSystem, "SystemID");
    expect(f, !v.messageheader.systemid.descriptivelabel.has_value(), "SystemID label absent");
    expect(f, v.messageheader.timestamp.value() == kStamp, "Timestamp");
    expect(f, v.messageheader.schemaversion.value() == kVersion, "SchemaVersion");
    expect(f, v.messageheader.mode == m::MessageModeEnum::LIVE, "Mode LIVE");
    expect(f, !v.messageheader.serviceid.has_value() && !v.messageheader.missionid.has_value(), "no Service/Mission");
}

/// The minimal document; `present` says whether UnassignAll was "" or absent.
inline std::vector<std::string> verify_minimal(const m::AMTISettingsCommandMT& v, bool present) {
    std::vector<std::string> f;
    verify_envelope(v, f);
    const auto& d = v.messagedata;
    expect(f, d.commandid.uuid.value() == kCommand, "CommandID");
    expect(f, d.commandstate == m::CommandStateEnum::NEW, "CommandState NEW");
    expect(f, d.subsystemid.uuid.value() == kSubsystem, "SubsystemID");
    expect(f, d.activatemdf.values().empty() && d.capabilityrank.values().empty() &&
                  d.capabilitystate.values().empty() && d.electronicprotectionoptions.values().empty() &&
                  d.messageoutput.values().empty(),
           "empty repetitions");
    expect(f, !d.representationkey && !d.emissionbalancestate && !d.defaultemcon, "absent optionals");
    if (present) {
        expect(f, d.unassignall.has_value(), "UnassignAll present");
        expect(f, d.unassignall && d.unassignall->value().empty(), "UnassignAll is the empty string");
        expect(f, d.unassignall && d.unassignall->value().size() == 0, "UnassignAll has length 0");
    } else {
        expect(f, !d.unassignall.has_value(), "UnassignAll absent");
    }
    return f;
}

inline std::vector<std::string> verify_full(const m::AMTISettingsCommandMT& v) {
    std::vector<std::string> f;
    verify_envelope(v, f);
    const auto& d = v.messagedata;
    expect(f, d.commandid.uuid.value() == kCommand, "CommandID");
    expect(f, d.commandid.descriptivelabel && d.commandid.descriptivelabel->value() == "AMTI settings #1",
           "CommandID label");
    expect(f, d.commandstate == m::CommandStateEnum::UPDATE, "CommandState UPDATE");
    expect(f, d.subsystemid.uuid.value() == kSubsystem, "SubsystemID");
    const auto& mdf = d.activatemdf.values();
    expect(f, mdf.size() == 1, "one ActivateMDF");
    if (!mdf.empty()) {
        expect(f, mdf[0].mdf_id.uuid.value() == kMdf, "MDF_ID");
        const auto& sub = mdf[0].subcategory.values();
        expect(f, sub.size() == 1 && sub[0].subcategoryid.uuid.value() == kSubCategory &&
                      sub[0].fileid.uuid.value() == kFile,
               "SubCategory");
    }
    const auto& rank = d.capabilityrank.values();
    expect(f, rank.size() == 1 && rank[0].capabilityid.uuid.value() == kCapability &&
                  rank[0].capabilityrank.value() == 65535U,
           "CapabilityRank 65535");
    const auto& state = d.capabilitystate.values();
    expect(f, state.size() == 1 && state[0].capabilitycommandstate == m::CapabilityCommandStateEnum::RESET,
           "CapabilityState RESET");
    expect(f, d.representationkey && d.representationkey->key.value() == "REP-1" &&
                  d.representationkey->systemname.value() == "Sys 1",
           "RepresentationKey");
    const auto& ep = d.electronicprotectionoptions.values();
    expect(f, ep.size() == 1, "one ElectronicProtectionOptions");
    if (!ep.empty()) {
        const auto& options = ep[0].electronicprotectionoptions.values();
        expect(f, options.size() == 1 && options[0].electronicprotectionoptionskey.key.value() == "EP-7" &&
                      options[0].electronicprotectionoptionskey.systemname.value() == "Jammer A" &&
                      options[0].electronicprotectionoptionstatus == m::EPCommandedStateEnum::DISABLE,
               "EP option");
    }
    expect(f, d.emissionbalancestate && *d.emissionbalancestate == m::EmissionBalanceEnum::BALANCED,
           "EmissionBalanceState");
    const auto& outputs = d.messageoutput.values();
    expect(f, outputs.size() == 2 && outputs[0] == MessageOutput::AMTIACTIVITY && outputs[1] == MessageOutput::ENTITY,
           "MessageOutput");
    expect(f, d.unassignall && d.unassignall->value().empty(), "UnassignAll present and empty");
    const auto* level = d.defaultemcon ? std::get_if<m::EmconSettingType::EmconLevel>(&d.defaultemcon->value) : nullptr;
    expect(f, level != nullptr && level->value == m::EmconLevelEnum::EMCONLEVEL3, "DefaultEmcon EMCON_LEVEL_3");
    return f;
}

}  // namespace amti
