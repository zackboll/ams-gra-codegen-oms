// CPP-OMS-006 model-level probe of the AUTHENTIC UCI FileMetadata closure.
//
// Compiled against the generated model header (oam.hpp) for production
// `service-generate` of FileMetadata, separately for UCI 2.5 and 2.6
// (-DUCI_FILE_METADATA_VERSION=25|26), WITHOUT --with-codec. With the flag,
// FileMetadata stays NOT READY in both releases: its one remaining codec
// blocker is the length-constrained hexBinary SHA_2_256_HashType. This probe
// therefore exercises only the authentic checked FileNameType carrier and
// where it sits in FileMetadataMDT. It claims nothing about JSON encoding,
// decoding or Sleet.
//
//   file_metadata_model_probe -> CPP_OMS_006_FILE_METADATA_MODEL uci=NN checks=N
//
// Behavioral reference: crates/runtime-rust-facade-tests/tests/real_uci_file_metadata.rs
// (report.tar.bz2 accepted; report. and an embedded TAB rejected). Every
// verdict below is derived from the XSD pattern
// [a-zA-Z0-9_\-]+\.[a-zA-Z0-9_\.\-]+ with minLength 1 / maxLength 255.
#include <iostream>
#include <optional>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>

#include "oam.hpp"

#ifndef UCI_FILE_METADATA_VERSION
#error "define UCI_FILE_METADATA_VERSION as 25 or 26"
#endif

namespace m = ::programs::oam;

static_assert(std::is_same<decltype(m::FileMetadataMDT::filename), m::FileNameType>::value,
              "FileMetadataMDT.FileName is the required checked FileNameType carrier");
static_assert(std::is_same<decltype(m::FileMetadataMT::messagedata), m::FileMetadataMDT>::value,
              "FileMetadataMT.MessageData");
static_assert(std::is_same<decltype(m::FileMetadataMDT::sha_2_hash), std::optional<m::SHA2256HashType>>::value,
              "the optional SHA_2_Hash member is the codec blocker");
static_assert(!std::is_default_constructible<m::FileNameType>::value &&
                  !std::is_constructible<m::FileNameType, std::string>::value &&
                  !std::is_constructible<m::FileNameType, std::string_view>::value &&
                  std::is_same<decltype(m::FileNameType::create(std::string_view())),
                               std::optional<m::FileNameType>>::value &&
                  std::is_same<decltype(std::declval<const m::FileNameType&>().value()), const std::string&>::value,
              "FileNameType is constructible only through create");

namespace {
int checks = 0;
int failures = 0;

void expect(std::string_view text, bool valid) {
    ++checks;
    const auto created = m::FileNameType::create(text);
    if (created.has_value() != valid || (created && created->value() != text)) {
        ++failures;
        std::cerr << "FileNameType::create(\"" << std::string(text) << "\") expected "
                  << (valid ? "accept" : "reject") << '\n';
    }
}
}  // namespace

int main() {
    for (const char* good : {"report.tar.bz2", "report.txt", "a.b", "x..", "_-.-_", "File-01_v2.LOG"}) {
        expect(good, true);
    }
    for (const char* bad : {"report.", ".txt", "report", "", "re port.txt", "report.txt ", "a/b.c", "a.b:c",
                            "r\xC3\xA9sum\xC3\xA9.txt"}) {
        expect(bad, false);
    }
    expect("report\t.txt", false);
    expect("report.txt\t", false);
    expect(std::string_view("a.b\0", 4), false);
    expect(std::string(127, 'n') + "." + std::string(127, 'e'), true);   // 255
    expect(std::string(128, 'n') + "." + std::string(127, 'e'), false);  // 256
    if (failures != 0) {
        std::cerr << failures << " failure(s) of " << checks << " checks\n";
        return 1;
    }
    std::cout << "CPP_OMS_006_FILE_METADATA_MODEL uci=" << UCI_FILE_METADATA_VERSION << " checks=" << checks
              << '\n';
    return 0;
}
