#define _GNU_SOURCE
#include <errno.h>
#include <fenv.h>
#include <float.h>
#include <limits.h>
#include <locale.h>
#include <math.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#if defined(__linux__) && defined(__GLIBC__)
#include <gnu/libc-version.h>
#endif
#if defined(__linux__) && defined(__x86_64__) && defined(__GLIBC__)
#include <xmmintrin.h>
/* Intel 64 and IA-32 Architectures SDM, Vol. 1, MXCSR register:
   bit 6 = Denormals-Are-Zero; bit 15 = Flush-To-Zero. This profile was
   qualified only with gradual underflow. Inspect controls on every call;
   never normalize caller controls, rounding, masks or status flags. */
enum { AMS_GRA_MXCSR_DAZ = 1u << 6, AMS_GRA_MXCSR_FTZ = 1u << 15 };
#endif

/* Status ABI: 0 success, 1 unsupported target/environment, 2 invalid input,
   3 resource failure. No borrowed pointers escape. No global locale changes. */
int ams_gra_float32_platform(void) {
#if defined(__linux__) && defined(__x86_64__) && defined(__GLIBC__) && \
    !defined(__FAST_MATH__)
    if ((_mm_getcsr() & (AMS_GRA_MXCSR_DAZ | AMS_GRA_MXCSR_FTZ)) != 0)
        return 1;
    if (strcmp(gnu_get_libc_version(), "2.41") != 0 || CHAR_BIT != 8 ||
        sizeof(float) != 4 || FLT_RADIX != 2 || FLT_MANT_DIG != 24 ||
        FLT_MIN_EXP != -125 || FLT_MAX_EXP != 128 ||
        sizeof(double) != 8 || DBL_MANT_DIG != 53 ||
        fegetround() != FE_TONEAREST)
        return 1;
    return 0;
#else
    return 1;
#endif
}

int ams_gra_float32_parse(const char *text, size_t length, size_t capacity,
                        float *output) {
    if (ams_gra_float32_platform() != 0) return 1;
    if (!text || !output || length == 0 || length > 1048576 ||
        capacity != length + 1 ||
        text[length] != '\0' || memchr(text, '\0', length) != NULL)
        return 2;
#if defined(__linux__) && defined(__x86_64__) && defined(__GLIBC__)
    locale_t locale = newlocale(LC_NUMERIC_MASK, "C", (locale_t)0);
    if (!locale) return 3;
    char *end = NULL;
    errno = 0;
    float value = strtof_l(text, &end, locale);
    /* ERANGE may mean correctly rounded subnormal/zero, not just overflow. */
    int status = end == text + length && isfinite(value) ? 0 : 2;
    freelocale(locale);
    if (status == 0) *output = value;
    return status;
#else
    return 1;
#endif
}

int ams_gra_float32_abi(float input, float *positive, float *negative_zero) {
    if (ams_gra_float32_platform() != 0) return 1;
    if (!positive || !negative_zero || input != 0x1.000002p0f) return 1;
    *positive = 0x1.000002p0f;
    *negative_zero = -0.0f;
    return 0;
}

int ams_gra_float32_special(int which, float *output) {
    if (ams_gra_float32_platform() != 0) return 1;
    if (!output || which < -1 || which > 1) return 2;
    *output = which == 0 ? NAN : which > 0 ? INFINITY : -INFINITY;
    return 0;
}