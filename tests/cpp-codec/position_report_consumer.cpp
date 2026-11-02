// CPP-OMS-004: generated C++17 codec for the AUTHENTIC pinned UCI
// PositionReport (payload PositionReportMT), UCI 2.5 or 2.6.
//
//   -DUCI_PR_VERSION=25|26, generated headers of that release
//   position_report_consumer <golden>
//
// <golden> is the release's pinned input (see position_report_documents.hpp).
// Prints `CPP_OMS_004_POSITION_REPORT uci=NN checks=N rejections=M`.
#include <cmath>
#include <fstream>
#include <iostream>
#include <sstream>
#include <string>
#include <string_view>

#include "oam.hpp"
#include "position_report_documents.hpp"
#include "service_api.hpp"
#include "service_codec.hpp"

namespace m = ::programs::oam;
namespace sc = ::service_codec;
namespace svc = ::service_api::function_loop;

namespace {

int failures = 0;
int checks = 0;
int rejections = 0;

#define CHECK(condition)                                                   \
    do {                                                                   \
        ++checks;                                                          \
        if (!(condition)) {                                                \
            ++failures;                                                    \
            std::cerr << __FILE__ << ':' << __LINE__                       \
                      << ": CHECK failed: " << #condition << '\n';         \
        }                                                                  \
    } while (false)

void report(const std::vector<std::string>& problems, const char* what) {
    ++checks;
    if (!problems.empty()) {
        ++failures;
        for (const std::string& problem : problems) {
            std::cerr << what << ": " << problem << '\n';
        }
    }
}

sc::CodecResult<m::PositionReportMT> decode(const std::string& json) { return sc::decode_PositionReportMT(json); }

std::string encode(const m::PositionReportMT& value) {
    const auto encoded = sc::encode_PositionReportMT(value);
    return encoded.ok() ? encoded.value() : "<encode failed at " + encoded.error().path + ": " + encoded.error().message + ">";
}

/// decode must succeed and re-encode to exactly `canonical`.
void canonical(const std::string& input, const std::string& canonical_text, const char* name) {
    const auto decoded = decode(input);
    ++checks;
    if (!decoded.ok()) {
        ++failures;
        std::cerr << name << ": decode failed at " << decoded.error().path << ": " << decoded.error().message << '\n';
        return;
    }
    const std::string again = encode(decoded.value());
    ++checks;
    if (again != canonical_text) {
        ++failures;
        std::cerr << name << ": re-encode differs\n  expected " << canonical_text << "\n  actual   " << again << '\n';
    }
}

const std::string kBase = "PositionReportMT.MessageData.InertialState.";

void reject(const std::string& json, const std::string& path, std::string_view message) {
    const auto decoded = decode(json);
    ++checks;
    ++rejections;
    if (decoded.ok()) {
        ++failures;
        std::cerr << "accepted but must be rejected (" << path << ")\n";
        return;
    }
    if (decoded.error().path != path || decoded.error().message.find(message) == std::string::npos) {
        ++failures;
        std::cerr << "wrong error\n  path    " << decoded.error().path << " (expected " << path << ")\n  message "
                  << decoded.error().message << " (expected ~" << message << ")\n";
    }
}

/// The full document with `from` (which must occur once) replaced by `to`.
std::string full_with(const std::string& from, const std::string& to) { return pr::with(pr::kFull, from, to); }

std::string read_file(const char* path) {
    std::ifstream in(path, std::ios::binary);
    if (!in) {
        std::cerr << "cannot read " << path << '\n';
        std::exit(2);
    }
    std::stringstream text;
    text << in.rdbuf();
    return text.str();
}

}  // namespace

int main(int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: position_report_consumer <golden>\n";
        return 2;
    }
    // Generated identity: the global wire element is PositionReport, the
    // payload type PositionReportMT.
    CHECK(svc::exchange_output_position::message_name == "PositionReport");
    CHECK(svc::exchange_input_position::message_name == "PositionReport");
    CHECK(svc::exchange_output_position::topic == "mission.position-report");

    // ---- 1. the pinned golden ----------------------------------------------
    const std::string golden = pr::payload_body(read_file(argv[1]));
    {
        const auto decoded = decode(golden);
        CHECK(decoded.ok());
        if (decoded.ok()) {
            report(pr::verify_golden(decoded.value()), "golden");
            CHECK(encode(decoded.value()) == pr::kGoldenCanonical);
        } else {
            std::cerr << "golden: " << decoded.error().path << ": " << decoded.error().message << '\n';
        }
        canonical(pr::kGoldenCanonical, pr::kGoldenCanonical, "golden canonical");
    }

    // ---- 2. every Float64 member, bit-exact ------------------------------------
    {
        const auto decoded = decode(pr::kFull);
        CHECK(decoded.ok());
        if (decoded.ok()) {
            report(pr::verify_full(decoded.value()), "full");
            CHECK(encode(decoded.value()) == pr::kFull);
        } else {
            std::cerr << "full: " << decoded.error().path << ": " << decoded.error().message << '\n';
        }
        const auto specials = decode(pr::kSpecials);
        CHECK(specials.ok());
        if (specials.ok()) {
            report(pr::verify_full(specials.value(), true), "specials");
            CHECK(encode(specials.value()) == pr::kSpecials);
        }
    }
    // Non-canonical but equal spellings decode to the same bits.
    canonical(full_with(R"("Latitude":1.5707963267948966)", R"("Latitude":15707963267948966e-16)"), pr::kFull,
              "latitude exponent spelling");
    canonical(full_with(R"("SemiMajorAxisLength":1e+21)", R"("SemiMajorAxisLength":1000000000000000000000.000)"),
              pr::kFull, "distance fixed spelling");
    canonical(full_with(R"("SemiMinorAxisLength":5e-324)", R"("SemiMinorAxisLength":2.4703282292062328e-324)"),
              pr::kFull, "subnormal rounding up");
    canonical(full_with(R"("UncertaintyTilt":1e-7)", R"("UncertaintyTilt":1.00000000000000000000001E-7)"),
              pr::kFull, "tilt long mantissa");
    canonical(full_with(R"("Roll":0.1)", R"("Roll":0.1000000000000000055511151231257827)"), pr::kFull,
              "roll exact binary value");
    canonical(full_with(R"("HAE_Adjustment":-0)", R"("HAE_Adjustment":-0.0e0)"), pr::kFull, "negative zero");
    // Correct underflow: Distance >= 0 admits both zero signs; the sign survives.
    canonical(full_with(R"("UncertaintyVertical":0)", R"("UncertaintyVertical":1e-400)"), pr::kFull,
              "underflow to +0");
    canonical(full_with(R"("UncertaintyVertical":0)", R"("UncertaintyVertical":-1e-400)"),
              full_with(R"("UncertaintyVertical":0)", R"("UncertaintyVertical":-0)"), "underflow to -0");

    // ---- 3. rejections (path + message) ----------------------------------------
    const std::string position = kBase + "Position.";
    reject(full_with("1.5707963267948966,\"Longitude\"", "1.5707963267948968,\"Longitude\""), position + "Latitude",
           "domain check");
    reject(full_with("\"Latitude\":1.5707963267948966", "\"Latitude\":\"NaN\""), position + "Latitude", "domain check");
    reject(full_with("\"Latitude\":1.5707963267948966", "\"Latitude\":\"Infinity\""), position + "Latitude",
           "domain check");
    reject(full_with("\"Longitude\":-3.141592653589793", "\"Longitude\":-3.1415926535897936"), position + "Longitude",
           "domain check");
    reject(full_with("\"Altitude\":-6378237", "\"Altitude\":-6378237.000000001"), position + "Altitude",
           "domain check");
    reject(full_with("\"Altitude\":-6378237", "\"Altitude\":\"-Infinity\""), position + "Altitude", "domain check");
    reject(full_with("\"Altitude\":-6378237", "\"Altitude\":\"NaN\""), position + "Altitude", "domain check");
    reject(full_with("\"SemiMajorAxisLength\":1e+21", "\"SemiMajorAxisLength\":-5e-324"),
           kBase + "PositionUncertainty.UncertaintyEllipse.SemiMajorAxisLength", "domain check");
    reject(full_with("\"UncertaintyVertical\":0", "\"UncertaintyVertical\":\"-Infinity\""),
           kBase + "PositionUncertainty.UncertaintyVertical", "domain check");
    reject(full_with("\"Yaw\":3.141592653589793", "\"Yaw\":3.1415926535897936"), kBase + "Orientation.Yaw",
           "domain check");
    reject(full_with("\"Pitch\":-1.5707963267948966", "\"Pitch\":-1.5707963267948968"), kBase + "Orientation.Pitch",
           "domain check");
    reject(full_with("\"WanderAngle\":0.9999999999999999", "\"WanderAngle\":\"Infinity\""),
           "PositionReportMT.MessageData.WanderAngle", "domain check");
    reject(full_with("\"NorthAcceleration\":9.80665", "\"NorthAcceleration\":1e309"),
           kBase + "DomainAcceleration.NorthAcceleration", "overflows binary64");
    reject(full_with("\"DownAcceleration\":100000000000000000000", "\"DownAcceleration\":-1.8e308"),
           kBase + "DomainAcceleration.DownAcceleration", "overflows binary64");
    const std::string speed = kBase + "DomainVelocity.NorthSpeed";
    const std::string north = "\"NorthSpeed\":2.2250738585072014e-308";
    for (const char* spelling : {"\"nan\"", "\"NAN\"", "\"inf\"", "\"+Infinity\"", "\"-NaN\"", "\"1.5\"", "\"\""}) {
        reject(full_with(north, std::string("\"NorthSpeed\":") + spelling), speed, "must be exactly \"NaN\"");
    }
    reject(full_with(north, "\"NorthSpeed\":true"), speed, "found boolean");
    reject(full_with(north, "\"NorthSpeed\":null"), speed, "found null");
    reject(full_with(north, "\"NorthSpeed\":[1]"), speed, "found array");
    reject(full_with(north, "\"NorthSpeed\":{}"), speed, "found object");
    reject(full_with("\"YawRate\":1.0000000000000002", "\"YawRate\":Infinity"), "$", "unexpected character");
    reject(full_with("\"YawRate\":1.0000000000000002", "\"YawRate\":01"), "$", "expected ',' or '}'");
    reject(full_with("\"YawRate\":1.0000000000000002", "\"YawRate\":1."), "$", "invalid number fraction");
    reject(full_with("\"Latitude\":1.5707963267948966,", ""), kBase + "Position", "missing required member \"Latitude\"");
    reject(full_with("\"Latitude\":1.5707963267948966,", "\"Latitude\":1,\"Latitude\":1,"), "$",
           "duplicate object member");
    reject(full_with("\"Roll\":0.1", "\"Roll\":0.1,\"roll\":0.1"), kBase + "Orientation", "unknown member \"roll\"");
    reject("{\"PositionReport\":" + pr::kFull + "}", "PositionReportMT", "unknown member \"PositionReport\"");
    // Integer semantics next to Float64 are unchanged.
    reject(full_with(R"("Timestamp":"2026-01-01T00:00:01Z"})", R"("Timestamp":"2026-01-01T00:00:01Z","SimulationTargetNumber":1.0})"),
           "PositionReportMT.MessageData.SimulationTargetNumber", "expected an integer");

    // ---- 4. encode-side: a carrier with forged bytes never leaves --------------
    {
        auto value = decode(pr::kFull).value();
        const double outside = 1.5707963267948968;
        std::memcpy(static_cast<void*>(&value.messagedata.inertialstate.position.latitude), &outside, sizeof outside);
        const auto encoded = sc::encode_PositionReportMT(value);
        ++checks;
        ++rejections;
        if (encoded.ok() || encoded.error().path != position + "Latitude" ||
            encoded.error().message.find("domain check") == std::string::npos) {
            ++failures;
            std::cerr << "forged Latitude was not refused\n";
        }
        auto specials = decode(pr::kFull).value();
        const double nan = std::nan("");
        std::memcpy(static_cast<void*>(&specials.messagedata.inertialstate.positionuncertainty->uncertaintyvertical),
                    &nan, sizeof nan);
        const auto refused = sc::encode_PositionReportMT(specials);
        ++checks;
        ++rejections;
        if (refused.ok() || refused.error().path != kBase + "PositionUncertainty.UncertaintyVertical") {
            ++failures;
            std::cerr << "forged UncertaintyVertical NaN was not refused\n";
        }
    }

    // ---- 5. robustness: truncations and single-byte corruptions never crash -----
    for (std::size_t length = 0; length < pr::kFull.size(); length += 7) {
        CHECK(!decode(pr::kFull.substr(0, length)).ok());
    }
    for (std::size_t i = 0; i < pr::kFull.size(); ++i) {
        for (const char replacement : {'\0', '-', '.', 'e', '9', '"'}) {
            std::string corrupt = pr::kFull;
            corrupt[i] = replacement;
            static_cast<void>(decode(corrupt));
        }
    }

    if (failures != 0) {
        std::cerr << failures << " failure(s)\n";
        return 1;
    }
    std::cout << "CPP_OMS_004_POSITION_REPORT uci=" << UCI_PR_VERSION << " checks=" << checks
              << " rejections=" << rejections << '\n';
    return 0;
}
