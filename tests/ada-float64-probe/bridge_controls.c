/* Fault injection for resource/error output atomicity. No production linkage. */
#define _GNU_SOURCE
#include <locale.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
extern int task096_parse(const char *, size_t, size_t, double *, size_t *, int *);
extern int task096_format(const double *, char *, size_t);
extern locale_t __real_newlocale(int, const char *, locale_t);
extern locale_t __real_uselocale(locale_t);
static int allocation_failure, switch_failure;
locale_t __wrap_newlocale(int mask, const char *name, locale_t base) {
    return allocation_failure ? (locale_t)0 : __real_newlocale(mask, name, base);
}
locale_t __wrap_uselocale(locale_t value) {
    if (switch_failure && value) return (locale_t)0;
    return __real_uselocale(value);
}
static unsigned checks;
#define CHECK(x) do { ++checks; if (!(x)) { fprintf(stderr,"FAIL line %d\n",__LINE__); exit(1); } } while (0)
int main(void) {
    double value = 42.0, input = 1.0;
    uint64_t bits;
    size_t consumed = 123;
    int error = 456;
    char buffer[32]; memset(buffer, 'X', sizeof buffer);
    locale_t previous = uselocale((locale_t)0);
    allocation_failure = 1;
    CHECK(task096_parse("1", 1, 2, &value, &consumed, &error) == 3);
    memcpy(&bits, &value, sizeof bits);
    CHECK(bits == UINT64_C(0x4045000000000000) && consumed == 123 && error == 456);
    CHECK(task096_format(&input, buffer, sizeof buffer) == 3);
    for (size_t i = 0; i < sizeof buffer; ++i) CHECK(buffer[i] == 'X');
    CHECK(uselocale((locale_t)0) == previous);
    allocation_failure = 0;
    switch_failure = 1;
    CHECK(task096_format(&input, buffer, sizeof buffer) == 3);
    switch_failure = 0;
    CHECK(uselocale((locale_t)0) == previous);
    for (size_t i = 0; i < sizeof buffer; ++i) CHECK(buffer[i] == 'X');
    CHECK(task096_format(&input, buffer, 1) == 2);
    CHECK(task096_parse("1", 1, 1, &value, &consumed, &error) == 2);
    CHECK(task096_parse("1\0x", 3, 4, &value, &consumed, &error) == 2);
    CHECK(task096_parse("1", 0, 1, &value, &consumed, &error) == 2);
    CHECK(task096_parse(NULL, 1, 2, &value, &consumed, &error) == 2);
    CHECK(task096_parse("nan", 3, 4, &value, &consumed, &error) == 2);
    CHECK(task096_parse("inf", 3, 4, &value, &consumed, &error) == 2);
    memcpy(&bits, &value, sizeof bits);
    CHECK(bits == UINT64_C(0x4045000000000000));
    CHECK(task096_format(&input, buffer, sizeof buffer) == 0);
    CHECK(!strcmp(buffer, "1") && uselocale((locale_t)0) == previous);
    CHECK(task096_parse("1", 1, 2, &value, &consumed, &error) == 0);
    printf("TASK096_BRIDGE_CONTROLS_COMPLETE checks=%u\n",checks);
    return 0;
}