// Independent authentic FileMetadata expectations, checked against pinned UCI
// 2.5/2.6. No generated JSON is used as the expected document.
#pragma once
#include "oam.hpp"
#include "service_codec.hpp"
#include <cstdlib>
#include <iostream>
#include <string>
#include <vector>

namespace file_metadata {
namespace m = programs::oam;
namespace sc = service_codec;
#if UCI_FILE_METADATA_VERSION == 25
inline const std::string version = "002.5.0";
#elif UCI_FILE_METADATA_VERSION == 26
inline const std::string version = "002.6.0";
#else
#error "define UCI_FILE_METADATA_VERSION=25 or 26"
#endif
inline const std::string uuid = "550e8400-e29b-41d4-a716-446655440007";
inline const std::string stamp = "2026-11-08T00:00:00Z";
inline const std::string security = R"({"Classification":"U","OwnerProducer":[{"GovernmentIdentifier":"USA"}]})";
inline std::string document(int payload, bool hash = true) {
    const std::string name = payload == 0 ? "report.tar.bz2" : "second.bin";
    const std::string digest = payload == 0 ? std::string(64, '0') : std::string(64, 'F');
    return R"({"SecurityInformation":)" + security + R"(,"MessageHeader":{"SystemID":{"UUID":")" + uuid +
        R"("},"Timestamp":")" + stamp + R"(","SchemaVersion":")" + version + R"(","Mode":"LIVE"},"MessageData":{"FileMetadataID":{"UUID":")" + uuid +
        R"("},"FileDescription":{"FileType":["OTHER"],"FileFormat":{"NonMIME":{"Key":"A","SystemName":"A"}}},"FileName":")" + name +
        R"(","FileSource":{},"CreationSource":"OTHER","UntrustedModification":)" + (payload == 0 ? "false" : "true") +
        R"(,"Timestamp":")" + stamp + R"(","SecurityInformation":)" + security +
        (hash ? R"(,"SHA_2_Hash":")" + digest + "\"" : "") + "}}";
}
inline m::FileMetadataMT decode(const std::string& json) {
    auto result = sc::decode_FileMetadataMT(json);
    if (!result.ok()) {
        std::cerr << result.error().path << ": " << result.error().message << '\n';
        std::exit(2);
    }
    return result.value();
}
inline m::FileMetadataMT make(int payload, bool hash = true) {
    // The seed supplies the large checked SecurityInformation aggregate. All
    // FileMetadata data and header fields below are constructed independently.
    const auto seed = decode(document(0, false));
    const auto id = *m::UniversallyUniqueIdentifierType::create(uuid);
    const auto time = *m::DateTimeType::create(stamp);
    m::FileMetadataMDT data{
        std::nullopt, m::FileMetadataIDType{id, std::nullopt},
        m::FileOutputType{*m::BoundedVector<m::FileTypeEnum, 1, 35>::create({m::FileTypeEnum::OTHER}),
            m::FileFormatType{m::FileFormatType::NonMIME{m::ForeignKeyType{
                *m::VisibleString256Type::create("A"), *m::VisibleString256Type::create("A")}}}},
        *m::FileNameType::create(payload == 0 ? "report.tar.bz2" : "second.bin"),
#if UCI_FILE_METADATA_VERSION == 26
        std::nullopt, // Authentic 2.6 adds optional FileSize before FileSource.
#endif
        m::SourceFiltersType{*m::UnboundedVector<m::SystemIDType, 0>::create({}),
            *m::UnboundedVector<m::SubsystemIDType, 0>::create({}),
            *m::UnboundedVector<m::ServiceIDType, 0>::create({}),
            *m::UnboundedVector<m::CapabilityIDType, 0>::create({})},
        std::nullopt, m::CreationSourceEnum::OTHER, payload != 0,
        time, seed.securityinformation, std::nullopt, *m::UnboundedVector<m::FileMetadataIDType, 0>::create({})};
    if (hash) data.sha_2_hash = *m::SHA2256HashType::create(
        std::vector<std::uint8_t>(32, payload == 0 ? std::uint8_t(0) : std::uint8_t(255)));
    return m::FileMetadataMT{seed.securityinformation,
        m::HeaderType{m::SystemIDType{id, std::nullopt}, time,
            *m::UCISchemaVersionStringType::create(version), m::MessageModeEnum::LIVE, std::nullopt, std::nullopt},
        std::nullopt, data};
}
inline bool verify(const m::FileMetadataMT& value, int payload, bool hash = true) {
    // Exact independent JSON comparison checks EVERY populated field, including
    // inherited security/header members, nested Choice, arrays and octets.
    const auto encoded = sc::encode_FileMetadataMT(value);
    const auto& d = value.messagedata;
    return encoded.ok() && encoded.value() == document(payload, hash) &&
        d.filename.value() == (payload == 0 ? "report.tar.bz2" : "second.bin") &&
        d.sha_2_hash.has_value() == hash && (!hash || d.sha_2_hash->value() ==
            std::vector<std::uint8_t>(32, payload == 0 ? std::uint8_t(0) : std::uint8_t(255)));
}
} // namespace file_metadata