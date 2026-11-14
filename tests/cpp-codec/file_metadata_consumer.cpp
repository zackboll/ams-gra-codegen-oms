#include "file_metadata_documents.hpp"
#include "check.hpp"
namespace fm = file_metadata;
TEST_CASE(authentic_roundtrips) {
    for (int payload : {0, 1}) {
        for (bool hash : {false, true}) {
            const auto value = fm::make(payload, hash);
            CHECK(fm::verify(value, payload, hash));
            CHECK(fm::verify(fm::decode(fm::document(payload, hash)), payload, hash));
            const auto encoded = fm::sc::encode_FileMetadataMT(value);
            CHECK(encoded.ok());
            CHECK_EQ(encoded.value(), fm::document(payload, hash));
        }
    }
}
TEST_CASE(authentic_hash_validation) {
    for (const std::size_t size : {std::size_t(0), std::size_t(31), std::size_t(32), std::size_t(33)}) {
        const auto carrier = fm::m::SHA2256HashType::create(std::vector<std::uint8_t>(size, 0));
        CHECK(carrier.has_value() == (size == 32));
        std::string json = fm::document(0);
        json.replace(json.find(std::string(64, '0')), 64, std::string(size * 2, '0'));
        const auto result = fm::sc::decode_FileMetadataMT(json);
        CHECK(result.ok() == carrier.has_value());
        if (!result.ok()) CHECK_EQ(result.error().path, std::string("FileMetadataMT.MessageData.SHA_2_Hash"));
    }
    const std::string original = "\"" + std::string(64, '0') + "\"";
    for (const std::string& token : {std::string("null"), std::string("42"), std::string("[]"),
            std::string("{}"), std::string("true"), "\"" + std::string(63, 'A') + "\"",
            "\"" + std::string(64, 'G') + "\""}) {
        std::string json = fm::document(0);
        json.replace(json.find(original), original.size(), token);
        const auto result = fm::sc::decode_FileMetadataMT(json);
        CHECK(!result.ok());
        CHECK_EQ(result.error().path, std::string("FileMetadataMT.MessageData.SHA_2_Hash"));
    }
    auto lowercase = fm::document(1);
    lowercase.replace(lowercase.find(std::string(64, 'F')), 64, std::string(64, 'f'));
    CHECK(fm::verify(fm::decode(lowercase), 1));
}
int main(int argc, char** argv) {
    return check::run_all(("CPP_OMS_007_FILE_METADATA uci=" + std::to_string(UCI_FILE_METADATA_VERSION)).c_str(), argc, argv);
}