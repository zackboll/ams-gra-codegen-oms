// Compile against the production headers generated for the combined Log/RDMA
// service. Input is the independently authored Task 092 RDMA golden document.
#include "service_api.hpp"
#include "service_codec.hpp"

#include <cstdlib>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

namespace m = programs::oam;
namespace sc = service_codec;
namespace rdma = service_api::function_rdma::exchange_receive;
namespace log_receive = service_api::function_log::exchange_receive;

static_assert(rdma::message_namespace == "https://www.vdl.afrl.af.mil/programs/oam");
static_assert(rdma::message_name == "RDMA_InitializeSetup" && rdma::topic == "uci.rdma");
static_assert(rdma::direction == "input" && std::is_same_v<rdma::Payload, m::RDMAInitializeSetupMT>);
static_assert(log_receive::message_namespace == rdma::message_namespace);
static_assert(log_receive::message_name == "Log" && log_receive::topic == "uci.log");
static_assert(log_receive::direction == "input" && std::is_same_v<log_receive::Payload, m::LogMT>);
static_assert(std::is_same_v<decltype(m::IPv4EndpointType::ipv4_address), m::IPv4AddressType>);
static_assert(std::is_same_v<decltype(m::IPv6EndpointType::ipv6_address), m::IPv6AddressType>);
static_assert(!std::is_constructible_v<m::IPv4AddressType, std::string>);
static_assert(!std::is_constructible_v<m::IPv6AddressType, std::string>);

namespace {

void require(bool condition, std::string_view detail) {
    if (!condition) {
        std::cerr << detail << '\n';
        std::exit(1);
    }
}

std::string replace(std::string text, std::string_view old, std::string_view replacement) {
    const auto index = text.find(old);
    require(index != std::string::npos && text.find(old, index + old.size()) == std::string::npos,
            "edit must match exactly one fixture token");
    text.replace(index, old.size(), replacement);
    return text;
}

void reject(std::string_view json, std::string_view member, std::string_view reason) {
    const auto decoded = sc::decode_RDMAInitializeSetupMT(json);
    require(!decoded, "invalid RDMA payload was accepted");
    require(decoded.error().path == std::string("RDMA_InitializeSetupMT.MessageData.") + std::string(member),
            decoded.error().path);
    require(decoded.error().message == reason, decoded.error().message);
}

void inspect(const m::RDMAInitializeSetupMT& message, std::string_view ipv6) {
    const auto& data = message.messagedata;
    require(data.rdma_provider == m::RDMAProviderEnum::TCP, "RDMA provider");
    const auto& endpoints = data.endpoint.values();
    require(endpoints.size() == 2, "expected two endpoints");
    const auto* v4 = std::get_if<m::NetworkEndpointType::IPv4Endpoint>(&endpoints[0].value);
    const auto* v6 = std::get_if<m::NetworkEndpointType::IPv6Endpoint>(&endpoints[1].value);
    require(v4 && v4->value.ipv4_address.value() == "192.0.2.10", "IPv4 endpoint or spelling");
    require(v6 && v6->value.ipv6_address.value() == ipv6, "IPv6 endpoint or spelling");
}

}  // namespace

int main(int argc, char** argv) {
    require(argc == 2, "expected authentic RDMA JSON fixture");
    std::ifstream input(argv[1]);
    require(input.is_open(), argv[1]);
    const std::string json{std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>()};
    const auto decoded = sc::decode_RDMAInitializeSetupMT(json);
    require(decoded.ok(), decoded.error().message);
    inspect(decoded.value(), "2001:db8::1");
    const auto encoded = sc::encode_RDMAInitializeSetupMT(decoded.value());
    require(encoded.ok(), encoded.error().message);
    const auto again = sc::decode_RDMAInitializeSetupMT(encoded.value());
    require(again.ok(), again.error().message);
    inspect(again.value(), "2001:db8::1");

    for (const std::string_view text : {"fE80::aBcD", ":001.009.099.199", "1::2::3"}) {
        const auto changed = sc::decode_RDMAInitializeSetupMT(replace(json, "2001:db8::1", text));
        require(changed.ok(), changed.error().message);
        inspect(changed.value(), text);
        const auto wire = sc::encode_RDMAInitializeSetupMT(changed.value());
        require(wire && wire.value() == replace(encoded.value(), "2001:db8::1", text), "IPv6 lexical round trip");
    }

    using Endpoints = decltype(m::RDMAInitializeSetupMDT::endpoint);
    static_assert(std::is_same_v<Endpoints, m::UnboundedVector<m::NetworkEndpointType, 0>>);
    const auto original = decoded.value().messagedata.endpoint.values();
    for (const std::size_t count : {0U, 1U, 4U}) {
        auto typed = decoded.value();
        std::vector<m::NetworkEndpointType> endpoints;
        for (std::size_t i = 0; i < count; ++i) {
            endpoints.push_back(original[i % original.size()]);
        }
        const auto checked = Endpoints::create(std::move(endpoints));
        require(checked.has_value(), "endpoint cardinality");
        typed.messagedata.endpoint = *checked;
        const auto wire = sc::encode_RDMAInitializeSetupMT(typed);
        require(wire.ok(), wire.error().message);
        const auto result = sc::decode_RDMAInitializeSetupMT(wire.value());
        require(result.ok(), result.error().message);
        const auto& got = result.value().messagedata.endpoint.values();
        require(got.size() == count, "round-trip endpoint cardinality");
        for (std::size_t i = 0; i < count; ++i) {
            require(got[i].value.index() == original[i % original.size()].value.index(), "endpoint Choice order");
        }
    }

    require(!m::IPv4AddressType::create("256.0.2.10"), "invalid checked IPv4 carrier");
    require(!m::IPv6AddressType::create("::ffff:256.0.2.128"), "invalid checked IPv6 carrier");
    reject(replace(json, "192.0.2.10", "256.0.2.10"), "Endpoint[0].IPv4_Endpoint.IPv4_Address",
           "value rejected by the generated model's IPv4_AddressType check");
    reject(replace(json, "2001:db8::1", "::ffff:256.0.2.128"), "Endpoint[1].IPv6_Endpoint.IPv6_Address",
           "value rejected by the generated model's IPv6_AddressType check");
    reject(replace(json, "\"2001:db8::1\"", "42"), "Endpoint[1].IPv6_Endpoint.IPv6_Address",
           "expected string, found number");

    // The harness compares this with the independently parsed golden JSON.
    std::cout << encoded.value() << '\n';
}