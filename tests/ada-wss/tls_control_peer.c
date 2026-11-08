/* Test-only TLS 1.2 peer. A verified WSS upgrade followed by HelloRequest
 * requires TLS-generated refusal output from OMS Poll, without Send. */
#include <openssl/ssl.h>
#include <openssl/err.h>
#include <openssl/sha.h>
#include <openssl/evp.h>
#include <openssl/sslerr.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define REQUIRE(x) do { if (!(x)) { ERR_print_errors_fp(stderr); fprintf(stderr, "failed: %s\n", #x); exit(1); } } while (0)

int main(int argc, char **argv) {
    REQUIRE(argc == 3);
    SSL_CTX *ctx = SSL_CTX_new(TLS_server_method());
    REQUIRE(ctx);
    REQUIRE(SSL_CTX_set_max_proto_version(ctx, TLS1_2_VERSION));
    REQUIRE(SSL_CTX_use_certificate_file(ctx, argv[1], SSL_FILETYPE_PEM) == 1);
    REQUIRE(SSL_CTX_use_PrivateKey_file(ctx, argv[2], SSL_FILETYPE_PEM) == 1);
    int listener = socket(AF_INET, SOCK_STREAM, 0);
    REQUIRE(listener >= 0);
    struct sockaddr_in addr = {.sin_family = AF_INET, .sin_addr.s_addr = htonl(INADDR_LOOPBACK)};
    REQUIRE(bind(listener, (struct sockaddr *)&addr, sizeof(addr)) == 0);
    REQUIRE(listen(listener, 1) == 0);
    socklen_t size = sizeof(addr);
    REQUIRE(getsockname(listener, (struct sockaddr *)&addr, &size) == 0);
    printf("PORT %u\n", ntohs(addr.sin_port)); fflush(stdout);
    int fd = accept(listener, NULL, NULL);
    REQUIRE(fd >= 0);
    struct timeval timeout = {.tv_sec = 8};
    REQUIRE(setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &timeout, sizeof(timeout)) == 0);
    SSL *ssl = SSL_new(ctx);
    REQUIRE(ssl);
    REQUIRE(SSL_set_fd(ssl, fd));
    REQUIRE(SSL_accept(ssl) == 1);
    char request[16384] = {0};
    size_t used = 0;
    while (!strstr(request, "\r\n\r\n")) {
        REQUIRE(used < sizeof(request) - 1);
        int n = SSL_read(ssl, request + used, (int)(sizeof(request) - 1 - used));
        REQUIRE(n > 0);
        used += (size_t)n;
    }
    char *key = strstr(request, "Sec-WebSocket-Key: ");
    REQUIRE(key);
    key += strlen("Sec-WebSocket-Key: ");
    char *end = strstr(key, "\r\n");
    REQUIRE(end && end - key == 24);
    char challenge[61];
    memcpy(challenge, key, 24);
    memcpy(challenge + 24, "258EAFA5-E914-47DA-95CA-C5AB0DC85B11", 37);
    unsigned char digest[SHA_DIGEST_LENGTH], encoded[29];
    REQUIRE(SHA1((unsigned char *)challenge, 60, digest));
    REQUIRE(EVP_EncodeBlock(encoded, digest, SHA_DIGEST_LENGTH) == 28);
    char response[512];
    int len = snprintf(response, sizeof(response), "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: %s\r\nSec-WebSocket-Protocol: owp\r\n\r\n", encoded);
    REQUIRE(SSL_write(ssl, response, len) == len);
    REQUIRE(SSL_renegotiate(ssl) == 1);
    REQUIRE(SSL_do_handshake(ssl) == 1);
    char byte;
    int ret = SSL_read(ssl, &byte, 1);
    REQUIRE(ret <= 0);
    unsigned long error = ERR_peek_last_error();
    REQUIRE(ERR_GET_REASON(error) == SSL_R_NO_RENEGOTIATION);
    puts("TLS_CONTROL_REPLY_RECEIVED plaintext=0"); fflush(stdout);
    SSL_free(ssl);
    SSL_CTX_free(ctx);
    close(fd); close(listener);
    return 0;
}