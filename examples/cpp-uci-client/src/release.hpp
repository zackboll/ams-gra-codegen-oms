// CPP-OMS-008: build-time binding of ONE authentic UCI release.
//
// build.sh compiles this application with
//   -DCPP_UCI_CLIENT_UCI_RELEASE=25|26
//   -DCPP_UCI_CLIENT_SCHEMA_SHA256="<SHA-256 of the MessageDefinitions root it generated from>"
// against the production-generated oam.hpp / service_api.hpp / service_codec.hpp
// for examples/cpp-uci-client/contracts/uci{25,26}-service.yaml.
//
// The release the binary advertises (OWP INIT `schema`, --help, banner) is a
// compile-time constant from this header ONLY, and every static_assert below
// must hold for the generated types actually compiled in. A binary built from
// UCI 2.6 types with the 2.5 selection (or vice versa), from a different
// contract, or from an unpinned schema does not compile. There is no run-time
// override.
#pragma once

#include <cstdint>
#include <string_view>
#include <type_traits>
#include <vector>

#include "oam.hpp"
#include "service_api.hpp"

#ifndef CPP_UCI_CLIENT_UCI_RELEASE
#error "build with examples/cpp-uci-client/build.sh (defines CPP_UCI_CLIENT_UCI_RELEASE=25 or 26)"
#endif
#ifndef CPP_UCI_CLIENT_SCHEMA_SHA256
#error "build with examples/cpp-uci-client/build.sh (defines CPP_UCI_CLIENT_SCHEMA_SHA256)"
#endif

namespace cpp_uci_client::release {

namespace m = ::programs::oam;
namespace endpoint = ::service_api::function_log::exchange_receive;

#if CPP_UCI_CLIENT_UCI_RELEASE == 25
inline constexpr std::string_view uci_release = "2.5";
inline constexpr std::string_view schema_version = "002.5.0";  // OWP INIT schema and MessageHeader.SchemaVersion
inline constexpr std::string_view pinned_root_sha256 =
    "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27";
// Generated-type fingerprint of the pinned UCI 2.5 Log closure.
using ExpectedDataCarrier = std::vector<std::uint8_t>;  // MessageData.Data: anonymous xs:hexBinary
using ExpectedUpTimeCarrier = m::XmlSchemaDuration;     // MessageData.ServiceUpTime: xs:duration
#elif CPP_UCI_CLIENT_UCI_RELEASE == 26
inline constexpr std::string_view uci_release = "2.6";
inline constexpr std::string_view schema_version = "002.6.0";
inline constexpr std::string_view pinned_root_sha256 =
    "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b";
// Generated-type fingerprint of the pinned UCI 2.6 Log closure.
using ExpectedDataCarrier = m::HexBinaryType;       // MessageData.Data: named HexBinaryType
using ExpectedUpTimeCarrier = m::DurationType;      // MessageData.ServiceUpTime: named DurationType
#else
#error "CPP_UCI_CLIENT_UCI_RELEASE must be 25 or 26"
#endif

inline constexpr std::string_view built_from_root_sha256 = CPP_UCI_CLIENT_SCHEMA_SHA256;
inline constexpr std::string_view oam_namespace = "https://www.vdl.afrl.af.mil/programs/oam";
inline constexpr std::string_view global_message = "Log";  // OWP global element; never "LogMT"
inline constexpr std::string_view application_topic = "uci.log";
inline constexpr std::string_view application_service = "cpp-uci-client";

static_assert(built_from_root_sha256 == pinned_root_sha256,
              "the generated types were not produced from the pinned MessageDefinitions root of the selected release");
static_assert(std::is_same_v<typename decltype(m::LogMDT::data)::value_type, ExpectedDataCarrier>,
              "generated LogMT does not match the selected UCI release (MessageData.Data carrier): "
              "regenerate with build.sh for the same release");
static_assert(std::is_same_v<typename decltype(m::LogMDT::serviceuptime)::value_type, ExpectedUpTimeCarrier>,
              "generated LogMT does not match the selected UCI release (MessageData.ServiceUpTime carrier): "
              "regenerate with build.sh for the same release");

// Generated endpoint identity of the receive-only application contract.
static_assert(std::is_same_v<endpoint::Payload, m::LogMT>, "the receive endpoint must carry programs::oam::LogMT");
static_assert(endpoint::message_name == global_message, "the OWP global message must be \"Log\", not \"LogMT\"");
static_assert(endpoint::message_namespace == oam_namespace, "Log must be in the OAM namespace");
static_assert(endpoint::topic == application_topic, "the application topic must be uci.log");
static_assert(endpoint::direction == std::string_view("input"), "the endpoint must be an input exchange");
static_assert(!endpoint::subscription_group.has_value(), "the application contract declares no subscription group");
static_assert(::service_api::service_name == application_service,
              "generated from a contract other than examples/cpp-uci-client/contracts/uci{25,26}-service.yaml");

}  // namespace cpp_uci_client::release
