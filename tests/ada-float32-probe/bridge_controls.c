#define _GNU_SOURCE
#include <fenv.h>
#include <locale.h>
#include <math.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

extern int ams_gra_float32_platform(void);
extern int ams_gra_float32_parse(const char *, size_t, size_t, float *);
extern int ams_gra_float32_abi(float, float *, float *);
extern locale_t __real_newlocale(int, const char *, locale_t);
extern const char *__real_gnu_get_libc_version(void);
static int fail_locale, unsupported;
locale_t __wrap_newlocale(int mask, const char *name, locale_t base) {
    return fail_locale ? (locale_t)0 : __real_newlocale(mask, name, base);
}
const char *__wrap_gnu_get_libc_version(void) {
    return unsupported ? "2.39" : __real_gnu_get_libc_version();
}

static unsigned checks;
#define REQUIRE(test) do { ++checks; if (!(test)) { fprintf(stderr, "failed line %d\n", __LINE__); return 1; } } while (0)
int main(void) {
    float output = 123.0f, negative_zero;
    REQUIRE(ams_gra_float32_platform() == 0);
    REQUIRE(ams_gra_float32_abi(0x1.000002p0f, &output, &negative_zero) == 0);
    REQUIRE(output == 0x1.000002p0f && negative_zero == 0 && signbit(negative_zero));
    REQUIRE(ams_gra_float32_abi(1.0f, &output, &negative_zero) == 1);
    REQUIRE(setlocale(LC_NUMERIC, "de_DE.utf8") != NULL);
    REQUIRE(ams_gra_float32_parse("0.1", 3, 4, &output) == 0 && output == 0.1f);
    REQUIRE(ams_gra_float32_parse("0,1", 3, 4, &output) == 2);
    REQUIRE(ams_gra_float32_parse("-0", 2, 3, &output) == 0 && signbit(output));
    REQUIRE(ams_gra_float32_parse("1e9999", 6, 7, &output) == 2);
    REQUIRE(ams_gra_float32_parse("-1e-9999", 8, 9, &output) == 0 && output == 0 && signbit(output));
    REQUIRE(ams_gra_float32_parse("0.1", 3, 3, &output) == 2);
    REQUIRE(ams_gra_float32_parse("0\0x", 3, 4, &output) == 2);
    REQUIRE(ams_gra_float32_parse(NULL, 1, 2, &output) == 2);
    fail_locale = 1;
    output = 123.0f;
    REQUIRE(ams_gra_float32_parse("0", 1, 2, &output) == 3 && output == 123.0f);
    fail_locale = 0;
    unsupported = 1;
    REQUIRE(ams_gra_float32_platform() == 1);
    REQUIRE(ams_gra_float32_parse("0", 1, 2, &output) == 1 && output == 123.0f);
    unsupported = 0;
    REQUIRE(fesetround(FE_UPWARD) == 0);
    REQUIRE(ams_gra_float32_parse("0", 1, 2, &output) == 1 && output == 123.0f);
    REQUIRE(fegetround() == FE_UPWARD);
    REQUIRE(fesetround(FE_TONEAREST) == 0);
    if (checks != 20) return 1;
    printf("TASK095_BRIDGE_CONTROLS_COMPLETE checks=%u\n", checks);
    return 0;
}