/* Test-only controls. Reuses the inspected Task095 harness, linked by runner. */
#define _GNU_SOURCE
#include <assert.h>
#include <errno.h>
#include <fenv.h>
#include <locale.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <xmmintrin.h>
extern int task096_platform(void);
extern int task096_parse(const char *, size_t, size_t, double *, size_t *, int *);
extern int task096_format(const double *, char *, size_t);
extern unsigned task095_x87(void);
extern unsigned task095_mxcsr(void);
extern int task095_rounding_save(void);
extern int task095_rounding_restore(void);
extern int task095_rounding_pair(unsigned, unsigned);
extern void task095_mode(unsigned);
static unsigned checks;
static int environment_saved;
#define CHECK(x) do { ++checks; if (!(x)) { if (environment_saved) (void)task095_rounding_restore(); fprintf(stderr,"FAIL line %d\n",__LINE__); exit(1); } } while (0)
static uint64_t bits(double value) {
    uint64_t result; memcpy(&result, &value, sizeof result); return result;
}
int main(int argc, char **argv) {
    CHECK(argc == 4);
    unsigned x87 = (unsigned)atoi(argv[1]), sse = (unsigned)atoi(argv[2]);
    unsigned flushing = (unsigned)atoi(argv[3]);
    unsigned original_x87 = task095_x87(), original_sse = task095_mxcsr();
    CHECK(task095_rounding_save() == 0);
    environment_saved = 1;
    CHECK(task095_rounding_pair(x87, sse) == 0);
    task095_mode(flushing);
    unsigned before_x87 = task095_x87(), before_sse = task095_mxcsr();
    int supported = x87 == 0 && sse == 0 && flushing == 0;
    double output = 42.0, input = 1.0;
    char text[32]; memset(text, 'X', sizeof text);
    size_t consumed = 999;
    int error = 777;
    int platform_status = task096_platform();
    int parse_status = task096_parse("1.0000000000000002", 18, 19, &output, &consumed, &error);
    int format_status = task096_format(&input, text, sizeof text);
    CHECK((platform_status == 0) == supported);
    CHECK(parse_status == (supported ? 0 : 1));
    CHECK(format_status == (supported ? 0 : 1));
    CHECK(task095_x87() == before_x87);
    /* Sticky exception flags may change for supported conversion. */
    CHECK((task095_mxcsr() & ~63u) == (before_sse & ~63u));
    if (!supported) {
        CHECK(bits(output) == UINT64_C(0x4045000000000000));
        CHECK(consumed == 999 && error == 777);
        for (size_t i = 0; i < sizeof text; ++i) CHECK(text[i] == 'X');
    } else {
        CHECK(bits(output) == UINT64_C(0x3ff0000000000001));
        CHECK(consumed == 18);
        CHECK(!strcmp(text, "1"));
    }
    CHECK(task095_rounding_restore() == 0);
    environment_saved = 0;
    CHECK(task095_x87() == original_x87 && task095_mxcsr() == original_sse);
    CHECK(task095_rounding_save() == 0);
    environment_saved = 1;
    CHECK(task096_parse("-0", 2, 3, &output, &consumed, &error) == 0);
    CHECK(bits(output) == UINT64_C(0x8000000000000000));
    CHECK(task096_parse("-1e-9999", 8, 9, &output, &consumed, &error) == 0);
    CHECK(bits(output) == UINT64_C(0x8000000000000000) && error == ERANGE);
    output = 42.0;
    CHECK(task096_parse("1e9999", 6, 7, &output, &consumed, &error) == 2);
    CHECK(bits(output) == UINT64_C(0x4045000000000000));
    CHECK(task096_parse("1x", 2, 3, &output, &consumed, &error) == 2 && consumed == 1);
    CHECK(bits(output) == UINT64_C(0x4045000000000000));
    /* Deliberately demonstrate C grammar is NOT a JSON validator. */
    CHECK(task096_parse("0x1p0", 5, 6, &output, &consumed, &error) == 0);
    CHECK(task096_parse("+1", 2, 3, &output, &consumed, &error) == 0);
    CHECK(task096_parse(" 1", 2, 3, &output, &consumed, &error) == 0);
    locale_t previous = uselocale((locale_t)0);
    locale_t german = newlocale(LC_NUMERIC_MASK, "de_DE.UTF-8", (locale_t)0);
    CHECK(german != (locale_t)0);
    CHECK(uselocale(german) == previous);
    CHECK(task096_parse("1.0000000000000002", 18, 19, &output, &consumed, &error) == 0);
    CHECK(bits(output) == UINT64_C(0x3ff0000000000001));
    CHECK(uselocale((locale_t)0) == german);
    CHECK(task096_format(&output, text, sizeof text) == 0);
    CHECK(!strcmp(text, "1.0000000000000002"));
    CHECK(uselocale((locale_t)0) == german);
    CHECK(uselocale(previous) == german);
    freelocale(german);
    CHECK(uselocale((locale_t)0) == previous);
    CHECK(task095_rounding_restore() == 0);
    environment_saved = 0;
    CHECK(task095_x87() == original_x87 && task095_mxcsr() == original_sse);
    printf("TASK096_C_ENVIRONMENT_COMPLETE x87=%u mxcsr=%u flushing=%u checks=%u\n", x87,sse,flushing,checks);
    return 0;
}