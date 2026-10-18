/* Test only, qualified glibc interposer. No Ada exception crosses C here.
   __libc_malloc/__libc_realloc avoid allocating dlsym/diagnostic recursion.
   An Ada wrapper arms just the next real allocator call, which receives NULL.
   System.Memory.Alloc then raises its own genuine Storage_Error. */
#define _GNU_SOURCE
#include <errno.h>
#include <stddef.h>
extern void *__libc_malloc(size_t);
extern void *__libc_realloc(void *, size_t);
static _Thread_local int armed;
static _Thread_local size_t failures, size;
void task097_c1_fail_next_malloc(void) { armed = 1; }
size_t task097_c1_malloc_failures(void) { return failures; }
size_t task097_c1_malloc_size(void) { return size; }
static int fail(size_t requested) {
    if (!armed) return 0;
    armed = 0;
    ++failures;
    size = requested;
    errno = ENOMEM;
    return 1;
}
void *malloc(size_t requested) {
    return fail(requested) ? NULL : __libc_malloc(requested);
}
void *realloc(void *pointer, size_t requested) {
    return fail(requested) ? NULL : __libc_realloc(pointer, requested);
}