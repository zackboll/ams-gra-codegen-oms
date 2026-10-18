#define _GNU_SOURCE
#include "ams_gra_float64.h"
#include <errno.h>
#include <fenv.h>
#include <float.h>
#include <limits.h>
#include <locale.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#if defined(__linux__) && defined(__GLIBC__)
#include <gnu/libc-version.h>
#endif
#if defined(__linux__) && defined(__x86_64__) && defined(__GLIBC__)
#include <xmmintrin.h>
#endif

int ams_gra_float64_platform(void) {
#if defined(__linux__) && defined(__x86_64__) && defined(__GLIBC__) && !defined(__FAST_MATH__)
    const unsigned mxcsr = _mm_getcsr();
    if ((mxcsr & ((1u << 6) | (1u << 15) | _MM_ROUND_MASK)) != 0 ||
        fegetround() != FE_TONEAREST || strcmp(gnu_get_libc_version(), "2.41") ||
        CHAR_BIT != 8 || sizeof(double) != 8 || FLT_RADIX != 2 ||
        DBL_MANT_DIG != 53 || DBL_MIN_EXP != -1021 || DBL_MAX_EXP != 1024)
        return 1;
    return 0;
#else
    return 1;
#endif
}

int ams_gra_float64_parse(const char *text, size_t length, size_t capacity,
                        double *output) {
    if (ams_gra_float64_platform()) return 1;
    if (!text || !output || !length || length > 1048576 ||
        capacity != length + 1 || text[length] || memchr(text, 0, length)) return 2;
#if defined(__linux__) && defined(__x86_64__) && defined(__GLIBC__)
    /* JSON grammar certification belongs to the Ada occurrence boundary. */
    locale_t locale = newlocale(LC_NUMERIC_MASK, "C", (locale_t)0);
    if (!locale) return 3;
    char *end = NULL;
    errno = 0;
    const double value = strtod_l(text, &end, locale);
    const int saved_errno = errno;
    const int status = end == text + length && isfinite(value) &&
        (saved_errno == 0 || saved_errno == ERANGE) ? 0 : 2;
    /* strtod_l does not install this locale. ERANGE with a finite result
       includes valid rounded subnormals and signed-zero underflow. */
    freelocale(locale);
    if (!status) *output = value;
    return status;
#else
    return 1;
#endif
}

int ams_gra_float64_format(const double *input, char *output, size_t capacity) {
    if (ams_gra_float64_platform()) return 1;
    if (!input || !output || !isfinite(*input) || !capacity) return 2;
#if defined(__linux__) && defined(__x86_64__) && defined(__GLIBC__)
    locale_t locale = newlocale(LC_NUMERIC_MASK, "C", (locale_t)0);
    if (!locale) return 3;
    locale_t previous = uselocale(locale);
    if (!previous) { freelocale(locale); return 3; }
    char buffer[32] = {0};
    const int length = snprintf(buffer, sizeof buffer, "%.17g", *input);
    /* Restore the EXACT previous handle, including LC_GLOBAL_LOCALE. On
       failure no safe ordinary return exists: callers could unknowingly
       retain our locale. Fail-stop, never free an installed locale, never
       publish. This branch is exercised in an isolated fault-test process. */
    if (uselocale(previous) != locale || uselocale((locale_t)0) != previous)
        abort();
    freelocale(locale);
    if (length < 1 || length >= (int)sizeof buffer ||
        buffer[length] != '\0' || memchr(buffer, 0, (size_t)length)) return 3;
    if ((size_t)length >= capacity) return 2;
    memcpy(output, buffer, (size_t)length + 1);
    return 0;
#else
    return 1;
#endif
}

int ams_gra_float64_special(int which, double *output) {
    if (ams_gra_float64_platform()) return 1;
    if (!output || which < -1 || which > 1) return 2;
    *output = which == 0 ? NAN : which > 0 ? INFINITY : -INFINITY;
    return 0;
}

int ams_gra_float64_abi(double input, double *positive, double *negative_zero) {
    if (ams_gra_float64_platform()) return 1;
    if (!positive || !negative_zero || input != 0x1.0000000000001p0) return 1;
    *positive = 0x1.0000000000001p0;
    *negative_zero = -0.0;
    return 0;
}