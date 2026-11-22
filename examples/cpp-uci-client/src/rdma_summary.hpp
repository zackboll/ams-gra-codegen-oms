// Only selected fields of the decoded RDMA_InitializeSetup payload are copied.
#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>
#include <string_view>
#include <variant>

#include "log_summary.hpp"
#include "oam.hpp"

namespace cpp_uci_client {

struct RdmaSummary {
    struct Endpoint {
        enum class Family { ipv4, ipv6 };
        Family family{};
        std::string address;
    };
    static constexpr std::size_t max_endpoints = 4;

    ::programs::oam::RDMAProviderEnum provider{};
    ::programs::oam::ClassificationEnum classification{};
    std::string setup_uuid;
    std::string schema_version;
    std::string header_timestamp;
    std::size_t endpoint_count = 0;
    std::array<Endpoint, max_endpoints> endpoints;

    std::size_t shown() const noexcept {
        return endpoint_count < endpoints.size() ? endpoint_count : endpoints.size();
    }
};

inline const char* provider_name(::programs::oam::RDMAProviderEnum value) noexcept {
    using E = ::programs::oam::RDMAProviderEnum;
    switch (value) {
        case E::VERBS: return "VERBS";
        case E::MLX: return "MLX";
        case E::TCP: return "TCP";
        case E::UDP: return "UDP";
    }
    return "UNRECOGNIZED";
}

inline RdmaSummary summarize_rdma(const ::programs::oam::RDMAInitializeSetupMT& setup) {
    namespace m = ::programs::oam;
    using Family = RdmaSummary::Endpoint::Family;
    RdmaSummary summary;
    const auto& data = setup.messagedata;
    summary.setup_uuid = data.rdma_initializesetupid.uuid.value();
    summary.provider = data.rdma_provider;
    summary.classification = setup.securityinformation.classification;
    summary.schema_version = setup.messageheader.schemaversion.value();
    summary.header_timestamp = setup.messageheader.timestamp.value().substr(0, timestamp_display_limit + 1);
    const auto& endpoints = data.endpoint.values();
    summary.endpoint_count = endpoints.size();
    for (std::size_t i = 0; i < summary.shown(); ++i) {
        const m::NetworkEndpointType& endpoint = endpoints[i];
        if (const auto* v4 = std::get_if<m::NetworkEndpointType::IPv4Endpoint>(&endpoint.value)) {
            const m::IPv4AddressType& address = v4->value.ipv4_address;
            summary.endpoints[i] = {Family::ipv4, address.value()};
        } else {
            const auto& v6 = std::get<m::NetworkEndpointType::IPv6Endpoint>(endpoint.value);
            const m::IPv6AddressType& address = v6.value.ipv6_address;
            summary.endpoints[i] = {Family::ipv6, address.value()};
        }
    }
    return summary;
}

inline std::string format_summary(std::uint64_t sequence, const RdmaSummary& s, std::string_view built_schema) {
    std::string line = "RECEIVED RDMA_InitializeSetup #" + std::to_string(sequence);
    line += " setup=" + bounded_text(s.setup_uuid, 36);
    line += " provider=";
    line += provider_name(s.provider);
    line += " classification=";
    line += classification_name(s.classification);
    line += " schema=" + bounded_text(s.schema_version, 57);
    line += " header-timestamp=" + bounded_text(s.header_timestamp, timestamp_display_limit);
    line += " endpoints=" + std::to_string(s.endpoint_count);
    for (std::size_t i = 0; i < s.shown(); ++i) {
        // The checked carriers store the original spelling; no normalization.
        line += s.endpoints[i].family == RdmaSummary::Endpoint::Family::ipv4 ? " ipv4=" : " ipv6=";
        line += s.endpoints[i].address;
    }
    if (s.shown() < s.endpoint_count) {
        line += " endpoints-omitted=" + std::to_string(s.endpoint_count - s.shown());
    }
    if (s.schema_version != built_schema) {
        line += " schema-differs-from-build=" + std::string(built_schema);
    }
    return line;
}

}  // namespace cpp_uci_client