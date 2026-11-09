// Independent octet boundaries; the codec must agree with carrier create().
#include "oam.hpp"
#include "service_codec.hpp"
#include <cstdlib>
#include <iostream>
#include <string>
#include <vector>

namespace m = programs::oam;
namespace sc = service_codec;
namespace {
int checks = 0;
void require(bool ok) {
    ++checks;
    if (!ok) { std::cerr << "boundary check " << checks << " failed\n"; std::exit(1); }
}
template <typename Carrier>
void boundary(const std::string& field, std::size_t size, bool valid) {
    const std::vector<std::uint8_t> bytes(size, std::uint8_t(255));
    const auto carrier = Carrier::create(bytes);
    require(carrier.has_value() == valid);
    const std::string digest(64, '0');
    const std::string prefix = "{\"Required\":\"" + digest + "\",\"Nested\":{\"Hash\":\"" + digest +
        "\"},\"Choice\":{\"Hash\":\"" + digest + "\"},\"" + field + "\":\"";
    const auto decoded = sc::decode_HexPayload(prefix + std::string(size * 2, 'f') + "\"}");
    require(decoded.ok() == valid);
    if (valid) {
        const auto& value = decoded.value();
        const auto& actual = field == "MinBytes" ? value.minbytes->value() :
            field == "MaxBytes" ? value.maxbytes->value() :
            field == "RangeBytes" ? value.rangebytes->value() : value.emptybytes->value();
        require(actual == carrier->value());
        const auto encoded = sc::encode_HexPayload(value);
        require(encoded.ok());
        require(encoded.value() == prefix + std::string(size * 2, 'F') + "\"}");
    } else {
        require(decoded.error().path == "HexPayload." + field);
    }
}
}
int main() {
    for (std::size_t size = 0; size <= 5; ++size) {
        boundary<m::MinBytes>("MinBytes", size, size >= 2);
        boundary<m::MaxBytes>("MaxBytes", size, size <= 4);
        boundary<m::RangeBytes>("RangeBytes", size, size >= 2 && size <= 4);
        boundary<m::EmptyBytes>("EmptyBytes", size, size == 0);
    }
    std::cout << "CPP_OMS_007_BOUNDARIES checks=" << checks << '\n';
}