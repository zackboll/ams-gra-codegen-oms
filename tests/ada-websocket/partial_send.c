/* Linux qualification-only syscall interposer. Never linked to production.
 * Force actual successful short socket writes; the peer still uses real TCP.
 */
#define _GNU_SOURCE
#include <sys/socket.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <stdio.h>

static unsigned shorts;

__attribute__((destructor)) static void evidence(void) {
    fprintf(stderr, "TASK100_FORCED_SHORT_SENDS=%u\n", shorts);
}

ssize_t send(int fd, const void *buf, size_t len, int flags) {
    size_t count = len > 1024 ? 1024 : len;
    if (count < len) shorts++;
    return syscall(SYS_sendto, fd, buf, count, flags, NULL, 0);
}

ssize_t sendto(int fd, const void *buf, size_t len, int flags,
               const struct sockaddr *addr, socklen_t addrlen) {
    size_t count = len > 1024 ? 1024 : len;
    if (count < len) shorts++;
    return syscall(SYS_sendto, fd, buf, count, flags, addr, addrlen);
}