#pragma once

// Private to the TLS-enabled runtime. Beast's SSL teardown composes TLS
// close_notify with WebSocket Close; do not start a separate SSL shutdown.
#include <boost/asio/ssl.hpp>
#include <boost/beast/websocket/ssl.hpp>

#include <openssl/err.h>
#include <openssl/pem.h>
#include <openssl/x509v3.h>

#include <filesystem>
#include <fstream>
#include <memory>
#include <stdexcept>
#include <string_view>

namespace ams_gra::lacal::detail {

// SSL_CTX_load_verify_file can accept a good certificate followed by bad PEM
// or unrelated text. Require a nonempty bundle of complete CA PEM blocks.
// Delimit the blocks here; OpenSSL alone decodes and interprets certificates.
inline bool add_ca_bundle(SSL_CTX* context, const std::string& path) {
    constexpr int max_size = 16 * 1024 * 1024;
    constexpr std::string_view begin = "-----BEGIN CERTIFICATE-----";
    constexpr std::string_view end = "-----END CERTIFICATE-----";
    std::error_code fs_error;
    if (path.find('\0') != std::string::npos || !std::filesystem::is_regular_file(path, fs_error)) {
        return false;
    }
    std::ifstream file(path, std::ios::binary | std::ios::ate);
    const auto size = file.tellg();
    if (size <= 0 || size > max_size) {
        return false;
    }
    std::string bytes(static_cast<std::size_t>(size), '\0');
    if (!file.seekg(0).read(bytes.data(), size) || file.peek() != std::char_traits<char>::eof()) {
        return false;
    }
    std::string_view rest(bytes);
    if (rest.find('\0') != std::string_view::npos) {
        return false;
    }
    bool added = false;
    for (;;) {
        const auto start = rest.find_first_not_of(" \t\r\n");
        if (start == std::string_view::npos) {
            return added;
        }
        rest.remove_prefix(start);
        if (rest.substr(0, begin.size()) != begin) {
            return false;
        }
        const auto marker = rest.find(end, begin.size());
        if (marker == std::string_view::npos) {
            return false;
        }
        const auto length = marker + end.size();
        const auto newline = rest.find('\n', length);
        // Supply OpenSSL with the original PEM lines, including an optional
        // final newline. No other object or nested BEGIN is allowed.
        if (rest.find(begin, begin.size()) < marker ||
            rest.substr(length, newline == std::string_view::npos ? newline : newline - length)
                .find_first_not_of(" \t\r") != std::string_view::npos) {
            return false;
        }
        const auto block_size = newline == std::string_view::npos ? rest.size() : newline + 1;
        const std::unique_ptr<BIO, decltype(&BIO_free)> bio(
            BIO_new_mem_buf(rest.data(), static_cast<int>(block_size)), BIO_free);
        const std::unique_ptr<X509, decltype(&X509_free)> cert(
            bio ? PEM_read_bio_X509(bio.get(), nullptr, nullptr, nullptr) : nullptr, X509_free);
        if (!cert || X509_check_ca(cert.get()) <= 0 ||
            X509_STORE_add_cert(SSL_CTX_get_cert_store(context), cert.get()) != 1) {
            ERR_clear_error();
            return false;
        }
        added = true;
        rest.remove_prefix(block_size);
    }
}

inline void configure_tls(boost::asio::ssl::context& context) {
    context.set_default_verify_paths();  // Additional anchors never replace these.
    context.set_verify_mode(boost::asio::ssl::verify_peer);
    auto* native = context.native_handle();
#ifdef SSL_OP_IGNORE_UNEXPECTED_EOF
    SSL_CTX_clear_options(native, SSL_OP_IGNORE_UNEXPECTED_EOF);
#endif
    if (SSL_CTX_set_min_proto_version(native, TLS1_2_VERSION) != 1 ||
        SSL_CTX_set_max_proto_version(native, TLS1_3_VERSION) != 1 ||
        SSL_CTX_set_cipher_list(native, "DEFAULT:!aNULL:!eNULL") != 1) {
        ERR_clear_error();
        throw std::runtime_error("cannot configure verified TLS policy");
    }
}

inline bool configure_identity(SSL* ssl, const std::string& host, bool ip_literal) {
    auto* parameters = SSL_get0_param(ssl);
    if (ip_literal) {
        // IP addresses must match an iPAddress SAN, never a DNS name or CN.
        return X509_VERIFY_PARAM_set1_ip_asc(parameters, host.c_str()) == 1;
    }
    SSL_set_hostflags(ssl, X509_CHECK_FLAG_NEVER_CHECK_SUBJECT | X509_CHECK_FLAG_NO_PARTIAL_WILDCARDS);
    // The SSL_set_tlsext_host_name macro expands to SSL_ctrl with a C cast.
    // Supply a writable, call-local copy to its equivalent typed call site.
    std::string sni(host);
    return SSL_set1_host(ssl, host.c_str()) == 1 &&
           SSL_ctrl(ssl, SSL_CTRL_SET_TLSEXT_HOSTNAME, TLSEXT_NAMETYPE_host_name, sni.data()) == 1;
}

}  // namespace ams_gra::lacal::detail