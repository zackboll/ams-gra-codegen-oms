/* Test-only real-socket interposer. Arm on first application output after
 * verified upgrade; refuse writes long enough to exercise continued Poll,
 * then send ciphertext three bytes at a time. Never linked to production. */
#define _GNU_SOURCE
#include <sys/socket.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>
#include <errno.h>
#include <time.h>

static unsigned blocked, partials;
static struct timespec armed;
static int started;

static size_t limit(size_t len) {
    if (!getenv("OMS_WSS_ARM_OUTPUT")) return len;
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    if (!started) { armed = now; started = 1; }
    double elapsed = now.tv_sec - armed.tv_sec + (now.tv_nsec - armed.tv_nsec) / 1e9;
    if (elapsed < 0.3) { blocked++; errno = EAGAIN; return 0; }
    if (len > 3) { partials++; return 3; }
    return len;
}

ssize_t send(int fd, const void *buf, size_t len, int flags) {
    size_t n = limit(len);
    if (!n && len) return -1;
    return syscall(SYS_sendto, fd, buf, n, flags, NULL, 0);
}

ssize_t sendto(int fd, const void *buf, size_t len, int flags,
               const struct sockaddr *addr, socklen_t addrlen) {
    size_t n = limit(len);
    if (!n && len) return -1;
    return syscall(SYS_sendto, fd, buf, n, flags, addr, addrlen);
}

__attribute__((destructor)) static void evidence(void) {
    fprintf(stderr, "WSS_BLOCKED_SENDS=%u WSS_PARTIAL_SENDS=%u\n", blocked, partials);
}