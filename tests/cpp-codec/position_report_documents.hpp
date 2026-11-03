// CPP-OMS-004: independently authored authentic UCI PositionReport documents
// and their typed expectations, shared by
//   tests/cpp-codec/position_report_consumer.cpp   (codec only)
//   tests/cpp-lacal/real_position_report_sleet.cpp (pinned Sleet)
//
// Every document here is written by hand in canonical form (schema order, no
// whitespace, the ECMAScript shortest spelling for every Float64), and every
// expected value is an IEEE binary64 bit pattern computed by the exact-rational
// oracle (tests/ada-float64-probe/oracle.py), not by the codec under test.
//
// Requires UCI_PR_VERSION=25|26 and the generated `oam.hpp` of that release.
#pragma once

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <string>
#include <variant>
#include <vector>

#include "oam.hpp"

#ifndef UCI_PR_VERSION
#error "define UCI_PR_VERSION=25 or 26"
#endif

namespace pr {

namespace m = ::programs::oam;

#if UCI_PR_VERSION == 25
inline const std::string kVersion = "002.5.0";
#elif UCI_PR_VERSION == 26
inline const std::string kVersion = "002.6.0";
#else
#error "UCI_PR_VERSION must be 25 or 26"
#endif

inline std::uint64_t bits(double value) {
    std::uint64_t out = 0;
    std::memcpy(&out, &value, sizeof out);
    return out;
}

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

// ---- the full document: every Float64 member of the closure ----------------
inline const std::string kSecurity =
    R"("SecurityInformation":{"Classification":"U","OwnerProducer":[{"GovernmentIdentifier":"USA"}]})";
inline const std::string kHeader =
    R"("MessageHeader":{"SystemID":{"UUID":"550e8400-e29b-41d4-a716-446655440000"},"Timestamp":"2026-01-01T00:00:00Z","SchemaVersion":")" +
    kVersion + R"(","Mode":"LIVE"})";
inline const std::string kPosition =
    R"("Position":{"Latitude":1.5707963267948966,"Longitude":-3.141592653589793,"Altitude":-6378237,"Timestamp":"2026-01-01T00:00:00Z","HAE_Adjustment":-0})";
inline const std::string kUncertainty =
    R"("PositionUncertainty":{"UncertaintyEllipse":{"SemiMajorAxisLength":1e+21,"SemiMinorAxisLength":5e-324,"Orientation":-0.5},"UncertaintyVertical":0,"UncertaintyTilt":1e-7})";
inline const std::string kDomainVelocity =
    R"("DomainVelocity":{"NorthSpeed":2.2250738585072014e-308,"EastSpeed":-1.7976931348623157e+308,"DownSpeed":0.30000000000000004})";
inline const std::string kGroundVelocity = R"("GroundVelocity":{"NorthSpeed":123.456,"EastSpeed":-0.000001})";
inline const std::string kAcceleration =
    R"("DomainAcceleration":{"NorthAcceleration":9.80665,"EastAcceleration":-2.225073858507201e-308,"DownAcceleration":100000000000000000000})";
inline const std::string kOrientation =
    R"("Orientation":{"Yaw":3.141592653589793,"Pitch":-1.5707963267948966,"Roll":0.1})";
inline const std::string kOrientationRate =
    R"("OrientationRate":{"YawRate":1.0000000000000002,"PitchRate":-1.5e+300,"RollRate":0.3333333333333333})";
inline const std::string kFull =
    "{" + kSecurity + "," + kHeader +
    R"(,"MessageData":{"SystemID":{"UUID":"550e8400-e29b-41d4-a716-446655440000"},"Source":"ACTUAL","CurrentOperatingDomain":"AIR","InertialState":{)" +
    kPosition + "," + kUncertainty + "," + kDomainVelocity + "," + kGroundVelocity + "," + kAcceleration + "," +
    kOrientation + "," + kOrientationRate +
    R"(},"WanderAngle":0.9999999999999999,"MagneticHeading":-3.141592653589793,"Timestamp":"2026-01-01T00:00:01Z"}})";

/// The same document with the three OMS special spellings in every
/// unconstrained Float64 kind and in the one-sided Altitude domain.
inline const std::string kSpecials = with(
    with(with(with(kFull, R"("Altitude":-6378237)", R"("Altitude":"Infinity")"),
              R"("HAE_Adjustment":-0)", R"("HAE_Adjustment":"-Infinity")"),
         R"("NorthSpeed":2.2250738585072014e-308,"EastSpeed":-1.7976931348623157e+308,"DownSpeed":0.30000000000000004)",
         R"("NorthSpeed":"NaN","EastSpeed":"Infinity","DownSpeed":"-Infinity")"),
    R"("NorthAcceleration":9.80665,"EastAcceleration":-2.225073858507201e-308)",
    R"("NorthAcceleration":"NaN","EastAcceleration":"-Infinity")");

struct Expect {
    const char* name;
    double value;
    std::uint64_t bits;
};

/// Bit-exact verification of the full document's Float64 members plus its
/// identity fields. Returns the failures (empty when everything matches).
inline std::vector<std::string> verify_full(const m::PositionReportMT& report, bool specials = false) {
    std::vector<std::string> failures;
    const auto expect = [&](bool ok, const std::string& what) {
        if (!ok) {
            failures.push_back(what);
        }
    };
    const auto& data = report.messagedata;
    const auto& state = data.inertialstate;
    expect(report.securityinformation.classification == m::ClassificationEnum::U, "Classification");
    expect(report.messageheader.schemaversion.value() == kVersion, "SchemaVersion");
    expect(report.messageheader.mode == m::MessageModeEnum::LIVE, "Mode");
    expect(data.systemid.uuid.value() == "550e8400-e29b-41d4-a716-446655440000", "SystemID");
    expect(data.source == m::SystemSourceEnum::ACTUAL, "Source");
    expect(data.currentoperatingdomain == m::EnvironmentEnum::AIR, "CurrentOperatingDomain");
    expect(data.timestamp && data.timestamp->value() == "2026-01-01T00:00:01Z", "MessageData.Timestamp");
    expect(state.position.timestamp.value() == "2026-01-01T00:00:00Z", "Position.Timestamp");
    if (!state.positionuncertainty || !state.domainvelocity || !state.groundvelocity || !state.domainacceleration ||
        !state.orientation || !state.orientationrate || !state.position.hae_adjustment ||
        !state.positionuncertainty->uncertaintyvertical || !state.positionuncertainty->uncertaintytilt ||
        !data.wanderangle || !data.magneticheading) {
        failures.push_back("an optional Float64 member is missing");
        return failures;
    }
    const auto& ellipse = state.positionuncertainty->uncertaintyellipse;
    std::vector<Expect> expected = {
        {"Latitude", state.position.latitude.value(), 0x3FF921FB54442D18ULL},
        {"Longitude", state.position.longitude.value(), 0xC00921FB54442D18ULL},
        {"Altitude", state.position.altitude.value(), specials ? 0x7FF0000000000000ULL : 0xC15854BF40000000ULL},
        {"HAE_Adjustment", state.position.hae_adjustment->value(),
         specials ? 0xFFF0000000000000ULL : 0x8000000000000000ULL},
        {"SemiMajorAxisLength", ellipse.semimajoraxislength.value(), 0x444B1AE4D6E2EF50ULL},
        {"SemiMinorAxisLength", ellipse.semiminoraxislength.value(), 0x0000000000000001ULL},
        {"Ellipse.Orientation", ellipse.orientation.value(), 0xBFE0000000000000ULL},
        {"UncertaintyVertical", state.positionuncertainty->uncertaintyvertical->value(), 0x0000000000000000ULL},
        {"UncertaintyTilt", state.positionuncertainty->uncertaintytilt->value(), 0x3E7AD7F29ABCAF48ULL},
        {"DomainVelocity.EastSpeed", state.domainvelocity->eastspeed.value(),
         specials ? 0x7FF0000000000000ULL : 0xFFEFFFFFFFFFFFFFULL},
        {"DomainVelocity.DownSpeed", state.domainvelocity->downspeed.value(),
         specials ? 0xFFF0000000000000ULL : 0x3FD3333333333334ULL},
        {"GroundVelocity.NorthSpeed", state.groundvelocity->northspeed.value(), 0x405EDD2F1A9FBE77ULL},
        {"GroundVelocity.EastSpeed", state.groundvelocity->eastspeed.value(), 0xBEB0C6F7A0B5ED8DULL},
        {"EastAcceleration", state.domainacceleration->eastacceleration.value(),
         specials ? 0xFFF0000000000000ULL : 0x800FFFFFFFFFFFFFULL},
        {"DownAcceleration", state.domainacceleration->downacceleration.value(), 0x4415AF1D78B58C40ULL},
        {"Yaw", state.orientation->yaw.value(), 0x400921FB54442D18ULL},
        {"Pitch", state.orientation->pitch.value(), 0xBFF921FB54442D18ULL},
        {"Roll", state.orientation->roll.value(), 0x3FB999999999999AULL},
        {"YawRate", state.orientationrate->yawrate.value(), 0x3FF0000000000001ULL},
        {"PitchRate", state.orientationrate->pitchrate.value(), 0xFE41EB2D66005835ULL},
        {"RollRate", state.orientationrate->rollrate.value(), 0x3FD5555555555555ULL},
        {"WanderAngle", data.wanderangle->value(), 0x3FEFFFFFFFFFFFFFULL},
        {"MagneticHeading", data.magneticheading->value(), 0xC00921FB54442D18ULL},
    };
    if (specials) {
        expect(std::isnan(state.domainvelocity->northspeed.value()), "DomainVelocity.NorthSpeed is not NaN");
        expect(std::isnan(state.domainacceleration->northacceleration.value()), "NorthAcceleration is not NaN");
    } else {
        expected.push_back(
            {"DomainVelocity.NorthSpeed", state.domainvelocity->northspeed.value(), 0x0010000000000000ULL});
        expected.push_back(
            {"NorthAcceleration", state.domainacceleration->northacceleration.value(), 0x40239D013A92A305ULL});
    }
    expect(!state.groundvelocity->downspeed.has_value(), "GroundVelocity.DownSpeed must be absent");
    for (const Expect& item : expected) {
        if (bits(item.value) != item.bits) {
            char text[96];
            std::snprintf(text, sizeof text, "%s bits %016llX != %016llX", item.name,
                          static_cast<unsigned long long>(bits(item.value)),
                          static_cast<unsigned long long>(item.bits));
            failures.emplace_back(text);
        }
    }
    return failures;
}

// ---- the pinned golden inputs ------------------------------------------------
// 2.5: tests/fixtures/oms-json/sleet-e38f61d8/PositionReport.json (global
//      element envelope, accepted by Sleet in Task 050).
// 2.6: tests/fixtures/python-whitespace-visible/goldens/2.6-PositionReport.json
//      (payload body, PY-016; additionally published to the 2.6 Sleet here).
#if UCI_PR_VERSION == 25
inline const std::string kGoldenCanonical =
    R"({"SecurityInformation":{"Classification":"U","OwnerProducer":[{"GovernmentIdentifier":"USA"}]},"MessageHeader":{"SystemID":{"UUID":"550e8400-e29b-41d4-a716-446655440000"},"Timestamp":"2026-01-01T00:00:00Z","SchemaVersion":"002.5.0","Mode":"LIVE"},"MessageData":{"SystemID":{"UUID":"550e8400-e29b-41d4-a716-446655440000"},"Source":"ACTUAL","CurrentOperatingDomain":"AIR","InertialState":{"Position":{"Latitude":1,"Longitude":1,"Altitude":1,"Timestamp":"2026-01-01T00:00:00Z"}}}})";
inline const std::uint64_t kGoldenLatitude = 0x3FF0000000000000ULL;   // 1.0
inline const std::uint64_t kGoldenLongitude = 0x3FF0000000000000ULL;  // 1.0
inline const std::uint64_t kGoldenAltitude = 0x3FF0000000000000ULL;   // 1.0
#else
inline const std::string kGoldenCanonical =
    R"({"SecurityInformation":{"Classification":"U","OwnerProducer":[{"GovernmentIdentifier":"USA"},{"NATO_SpecialWord":"NATO:EXAMPLE"}],"ClassifiedBy":" a\n\r  b "},"MessageHeader":{"SystemID":{"UUID":"12345678-1234-1234-8234-123456789abc"},"Timestamp":"2026-01-02T03:04:05Z","SchemaVersion":"002.6.0","Mode":"LIVE"},"MessageData":{"SystemID":{"UUID":"12345678-1234-1234-8234-123456789abc"},"Source":"ACTUAL","CurrentOperatingDomain":"AIR","InertialState":{"Position":{"Latitude":0.5,"Longitude":1,"Altitude":100,"Timestamp":"2026-01-02T03:04:05Z"}}}})";
inline const std::uint64_t kGoldenLatitude = 0x3FE0000000000000ULL;   // 0.5
inline const std::uint64_t kGoldenLongitude = 0x3FF0000000000000ULL;  // 1.0
inline const std::uint64_t kGoldenAltitude = 0x4059000000000000ULL;   // 100.0
#endif

/// Strip a `{"PositionReport": BODY}` global-element envelope (pretty-printed
/// or compact); a bare body is returned unchanged.
inline std::string payload_body(const std::string& text) {
    const std::size_t open = text.find('{');
    const std::size_t key = text.find("\"PositionReport\"");
    if (open == std::string::npos || key == std::string::npos || text.find('{', open + 1) < key) {
        return text;
    }
    const std::size_t colon = text.find(':', key);
    const std::size_t close = text.rfind('}');
    return text.substr(colon + 1, close - colon - 1);
}

/// Typed checks of a decoded golden (independent of the encoder).
inline std::vector<std::string> verify_golden(const m::PositionReportMT& report) {
    std::vector<std::string> failures;
    const auto expect = [&](bool ok, const char* what) {
        if (!ok) {
            failures.emplace_back(what);
        }
    };
    const auto& security = report.securityinformation;
    expect(security.classification == m::ClassificationEnum::U, "Classification");
    expect(!security.ownerproducer.values().empty(), "OwnerProducer");
    if (!security.ownerproducer.values().empty()) {
        const auto* government =
            std::get_if<m::OwnerProducerChoiceType::GovernmentIdentifier>(&security.ownerproducer.values()[0].value);
        expect(government != nullptr && government->value == m::OwnerProducerEnum::USA, "OwnerProducer[0]");
    }
    expect(report.messageheader.schemaversion.value() == kVersion, "SchemaVersion");
    expect(report.messageheader.mode == m::MessageModeEnum::LIVE, "Mode");
    const auto& data = report.messagedata;
    expect(data.source == m::SystemSourceEnum::ACTUAL, "Source");
    expect(data.currentoperatingdomain == m::EnvironmentEnum::AIR, "CurrentOperatingDomain");
    expect(bits(data.inertialstate.position.latitude.value()) == kGoldenLatitude, "Latitude bits");
    expect(bits(data.inertialstate.position.longitude.value()) == kGoldenLongitude, "Longitude bits");
    expect(bits(data.inertialstate.position.altitude.value()) == kGoldenAltitude, "Altitude bits");
    expect(!data.inertialstate.position.hae_adjustment && !data.inertialstate.domainvelocity &&
               !data.inertialstate.positionuncertainty && !data.wanderangle,
           "golden has no other Float64 member");
#if UCI_PR_VERSION == 26
    expect(security.ownerproducer.values().size() == 2, "OwnerProducer size");
    expect(security.classifiedby && security.classifiedby->value() == " a\n\r  b ",
           "ClassifiedBy is preserved verbatim in 2.6");
#endif
    return failures;
}

}  // namespace pr
