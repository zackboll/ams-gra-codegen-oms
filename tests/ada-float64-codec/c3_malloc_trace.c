/* Test-only glibc allocation inventory. Fixed storage; no dlsym, backtrace,
   formatting, or allocation occurs in the armed instrumentation. */
#define _GNU_SOURCE
#include <errno.h>
#include <stddef.h>
#include <stdint.h>
extern void *__libc_malloc(size_t);
extern void *__libc_realloc(void *, size_t);
struct event { size_t bytes; uintptr_t caller; int reallocating; };
static _Thread_local struct event events[65536];
static _Thread_local size_t count, remaining, failures, failed_size;
static _Thread_local int active;
void task097_c3_start(size_t after) {
    count = failures = failed_size = 0;
    remaining = after;
    active = 1;
}
void task097_c3_stop(void) { active = 0; }
size_t task097_c3_count(void) { return count; }
size_t task097_c3_failures(void) { return failures; }
size_t task097_c3_size(void) { return failed_size; }
size_t task097_c3_event_size(size_t i) { return events[i].bytes; }
uintptr_t task097_c3_event_caller(size_t i) { return events[i].caller; }
int task097_c3_event_realloc(size_t i) { return events[i].reallocating; }
static int observe(size_t bytes, uintptr_t caller, int reallocating) {
    if (!active) return 0;
    if (count < 65536) events[count] = (struct event){bytes, caller, reallocating};
    ++count;
    if (remaining) { --remaining; return 0; }
    active = 0; /* Real allocator NULL is one-shot, before any unwinding. */
    ++failures;
    failed_size = bytes;
    errno = ENOMEM;
    return 1;
}
void *malloc(size_t bytes) {
    return observe(bytes, (uintptr_t)__builtin_return_address(0), 0)
        ? NULL : __libc_malloc(bytes);
}
void *realloc(void *p, size_t bytes) {
    return observe(bytes, (uintptr_t)__builtin_return_address(0), 1)
        ? NULL : __libc_realloc(p, bytes);
}