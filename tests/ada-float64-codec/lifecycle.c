#define _GNU_SOURCE
#include "ams_gra_float64.h"
#include <locale.h>
#include <pthread.h>
#include <signal.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>

extern locale_t __real_newlocale(int, const char *, locale_t);
extern locale_t __real_uselocale(locale_t);
extern int __real_vsnprintf(char *, size_t, const char *, va_list);
static _Thread_local int fault, selections;
locale_t __wrap_newlocale(int mask, const char *name, locale_t base) {
    return fault == 1 ? (locale_t)0 : __real_newlocale(mask, name, base);
}
locale_t __wrap_uselocale(locale_t value) {
    if (value) {
        ++selections;
        if ((fault == 2 && selections == 1) || (fault == 5 && selections == 2))
            return (locale_t)0;
    }
    return __real_uselocale(value);
}
int __wrap_snprintf(char *buffer, size_t size, const char *format, ...) {
    if (fault == 3) return -1;
    if (fault == 4) return (int)size;
    va_list args;
    va_start(args, format);
    int result = __real_vsnprintf(buffer, size, format, args);
    va_end(args);
    if (fault == 6 && result > 0 && (size_t)result < size) buffer[result] = 'X';
    return result;
}
#define CHECK(x) do { if (!(x)) { fprintf(stderr, "FAIL %d\n", __LINE__); exit(1); } } while (0)
static uint64_t bits(double value) {
    uint64_t result; memcpy(&result, &value, sizeof result); return result;
}
static void *thread(void *name) {
    locale_t locale = newlocale(LC_NUMERIC_MASK, name, (locale_t)0);
    CHECK(locale);
    locale_t previous = uselocale(locale);
    CHECK(previous);
    for (unsigned i = 0; i < 1000; ++i) {
        double value = 0;
        char output[32];
        CHECK(!ams_gra_float64_parse("1.0000000000000002", 18, 19, &value));
        CHECK(bits(value) == UINT64_C(0x3ff0000000000001));
        CHECK(!ams_gra_float64_format(&value, output, sizeof output));
        CHECK(!strcmp(output, "1.0000000000000002"));
        CHECK(uselocale((locale_t)0) == locale);
    }
    CHECK(uselocale(previous) == locale);
    freelocale(locale);
    CHECK(uselocale((locale_t)0) == previous);
    return NULL;
}
int main(void) {
    double value = 42.0, input = 1.0;
    locale_t previous = uselocale((locale_t)0);
    for (int mode = 1; mode <= 6; ++mode) {
        if (mode == 5) continue;
        char output[32]; memset(output, 'X', sizeof output);
        fault = mode; selections = 0;
        CHECK(ams_gra_float64_format(&input, output, sizeof output) == 3);
        for (size_t i = 0; i < sizeof output; ++i) CHECK(output[i] == 'X');
        CHECK(__real_uselocale((locale_t)0) == previous);
        if (mode == 1) {
            CHECK(ams_gra_float64_parse("1", 1, 2, &value) == 3);
            CHECK(bits(value) == UINT64_C(0x4045000000000000));
        }
        fault = 0;
    }
    /* Restoration failure is a documented fail-stop contract, not a returned
       resource error. Isolate it and prohibit core files. */
    pid_t child = fork(); CHECK(child >= 0);
    if (!child) {
        struct rlimit zero = {0, 0}; setrlimit(RLIMIT_CORE, &zero);
        fault = 5; selections = 0;
        char output[32];
        (void)ams_gra_float64_format(&input, output, sizeof output);
        _exit(99);
    }
    int status; CHECK(waitpid(child, &status, 0) == child);
    CHECK(WIFSIGNALED(status) && WTERMSIG(status) == SIGABRT);
    char output[32];
    CHECK(!ams_gra_float64_format(&input, output, sizeof output));
    CHECK(!strcmp(output, "1") && uselocale((locale_t)0) == previous);
    pthread_t threads[4];
    for (int i = 0; i < 4; ++i)
        CHECK(!pthread_create(&threads[i], NULL, thread, i % 2 ? "C" : "de_DE.UTF-8"));
    for (int i = 0; i < 4; ++i) CHECK(!pthread_join(threads[i], NULL));
    CHECK(uselocale((locale_t)0) == previous);
    puts("TASK097_C_LIFECYCLE_COMPLETE faults=6 concurrent_calls=8000 failstop=SIGABRT");
    return 0;
}