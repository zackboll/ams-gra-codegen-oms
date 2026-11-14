// CPP-OMS-007: independent wire expectations and checked-carrier agreement.
#include "oam.hpp"
#include "service_codec.hpp"
#include <cstdlib>
#include <iostream>
#include <string>
#include <type_traits>
#include <vector>

namespace m = programs::oam;
namespace sc = service_codec;
static_assert(!std::is_default_constructible<m::Digest>::value, "checked carrier");
static_assert(!std::is_constructible<m::Digest, std::vector<std::uint8_t>>::value, "private constructor");
static_assert(std::is_same<decltype(std::declval<const m::Digest&>().value()),
                           const std::vector<std::uint8_t>&>::value, "validated octets");
namespace {
int checks = 0;
int rejections = 0;
void require(bool ok) {
    ++checks;
    if (!ok) { std::cerr << "check " << checks << " failed\n"; std::exit(1); }
}
std::string quoted(const std::string& s) { return "\"" + s + "\""; }
const std::string good(64, 'A');
std::string document(const std::string& token, int position, bool optional = false) {
    const std::string valid = quoted(good);
    return "{\"Required\":" + (position == 0 ? token : valid) +
        (optional ? ",\"Optional\":" + token : "") +
        ",\"Nested\":{\"Hash\":" + (position == 1 ? token : valid) +
        "},\"Choice\":" + (position == 3 ? "{\"Record\":{\"Hash\":" + token + "}}" :
                              "{\"Hash\":" + (position == 2 ? token : valid) + "}") + "}";
}
void reject(const std::string& json, const std::string& path) {
    const auto result = sc::decode_HexPayload(json);
    require(!result.ok());
    if (result.error().path != path) std::cerr << "expected " << path << " got " << result.error().path << '\n';
    require(result.error().path == path);
    ++rejections;
}
}
int main() {
    const std::vector<std::string> paths = {"HexPayload.Required", "HexPayload.Nested.Hash",
        "HexPayload.Choice.Hash", "HexPayload.Choice.Record.Hash"};
    // Independently supplied octets, not obtained using the codec parser.
    for (const std::uint8_t byte : {std::uint8_t(0), std::uint8_t(170), std::uint8_t(255)}) {
        const std::string upper = byte == 0 ? std::string(64, '0') : std::string(64, byte == 170 ? 'A' : 'F');
        const std::string lower = byte == 0 ? upper : std::string(64, byte == 170 ? 'a' : 'f');
        const auto carrier = m::Digest::create(std::vector<std::uint8_t>(32, byte));
        require(carrier.has_value());
        for (const std::string& input : {upper, lower, " \\t" + lower + "\\r\\n "}) {
            for (int pos = 0; pos < 4; ++pos) {
                auto decoded = sc::decode_HexPayload(document(quoted(input), pos));
                require(decoded.ok());
                if (pos == 0) require(decoded.value().required.value() == carrier->value());
                auto encoded = sc::encode_HexPayload(decoded.value());
                require(encoded.ok());
                require(encoded.value() == document(quoted(upper), pos));
            }
        }
        auto optional = sc::decode_HexPayload(document(quoted(lower), 4, true));
        require(optional.ok() && optional.value().optional.has_value());
        require(optional.value().optional->value() == carrier->value());
        require(sc::encode_HexPayload(optional.value()).value() == document(quoted(upper), 4, true));
        auto absent = sc::decode_HexPayload(document(quoted(upper), 4));
        require(absent.ok() && !absent.value().optional.has_value());
        // Copy-only carrier lifecycle must not invalidate a moved-from value.
        auto source = *carrier;
        const auto moved = std::move(source);
        require(source.value() == carrier->value() && moved.value() == carrier->value());
    }
    for (const std::size_t size : {std::size_t(0), std::size_t(31), std::size_t(33)}) {
        require(!m::Digest::create(std::vector<std::uint8_t>(size, 0)).has_value());
        const std::string token = quoted(std::string(size * 2, '0'));
        for (int pos = 0; pos < 4; ++pos) reject(document(token, pos), paths[static_cast<std::size_t>(pos)]);
        reject(document(token, 4, true), "HexPayload.Optional");
    }
    for (const std::string& token : {quoted(std::string(63, 'A')), quoted(std::string(64, 'G')),
             quoted("AA AA"), quoted("0x" + std::string(62, '0')), std::string("null"),
             std::string("42"), std::string("true"), std::string("[]"), std::string("{}")}) {
        for (int pos = 0; pos < 4; ++pos) reject(document(token, pos), paths[static_cast<std::size_t>(pos)]);
        reject(document(token, 4, true), "HexPayload.Optional");
    }
    reject("{}", "HexPayload");
    std::string missing = document(quoted(good), 0);
    const auto nested = missing.find("\"Nested\":{\"Hash\":");
    missing.replace(nested, std::string("\"Nested\":{\"Hash\":" + quoted(good) + "}").size(), "\"Nested\":{}");
    reject(missing, "HexPayload.Nested");
    reject("null", "HexPayload");
    std::string null_nested = document(quoted(good), 0);
    null_nested.replace(null_nested.find("\"Nested\":{\"Hash\":"),
        std::string("\"Nested\":{\"Hash\":" + quoted(good) + "}").size(), "\"Nested\":null");
    reject(null_nested, "HexPayload.Nested");
    // Parser rejects duplicate keys before semantic decoding, at the duplicate's path.
    reject("{\"Required\":" + quoted(good) + ",\"Required\":" + quoted(good) + "}", "$");
    reject("{\"Nested\":{\"Hash\":" + quoted(good) + ",\"Hash\":" + quoted(good) + "}}", "$");
    reject("{\"Choice\":{\"Hash\":" + quoted(good) + ",\"Hash\":" + quoted(good) + "}}", "$");
    std::cout << "CPP_OMS_007_HEX checks=" << checks << " rejections=" << rejections << '\n';
}