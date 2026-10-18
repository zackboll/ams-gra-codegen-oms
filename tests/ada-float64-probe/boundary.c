/* Handwritten Task096 research boundary. Not linked into production. */
#define _GNU_SOURCE
#include <errno.h>
#include <fenv.h>
#include <float.h>
#include <gnu/libc-version.h>
#include <limits.h>
#include <locale.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <xmmintrin.h>

/* Same inspected MXCSR contract as Task095, independently tested below.
   No floating expressions precede approval; do not cache approval. */
int task096_platform(void) {
#if defined(__linux__) && defined(__x86_64__) && !defined(__FAST_MATH__)
    unsigned mxcsr = _mm_getcsr();
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

/* Status: 0 finite success, 1 unsupported, 2 invalid/overflow, 3 resource.
   JSON syntax is the caller's SAX responsibility, not strtod_l's grammar. */
int task096_parse(const char *text, size_t length, size_t capacity,
                  double *output, size_t *consumed, int *error) {
    if (task096_platform()) return 1;
    if (!text || !output || !consumed || !error || !length || length > 1048576 ||
        capacity != length + 1 || text[length] || memchr(text, 0, length)) return 2;
    locale_t locale = newlocale(LC_NUMERIC_MASK, "C", (locale_t)0);
    if (!locale) return 3;
    char *end = NULL;
    errno = 0;
    double value = strtod_l(text, &end, locale);
    int saved_error = errno;
    size_t used = (size_t)(end - text);
    int status = used == length && isfinite(value) &&
                 (saved_error == 0 || saved_error == ERANGE) ? 0 : 2;
    freelocale(locale);
    *consumed = used;
    *error = saved_error;
    if (!status) *output = value;
    return status;
}

/* 17 significant decimal digits suffice for binary64 round-trip, subject to
   executed libc qualification. Bounded buffer; exact previous thread locale
   restored before publication. No parse-through-DOM is allowed afterward. */
int task096_format(const double *input, char *output, size_t capacity) {
    if (task096_platform()) return 1;
    if (!input || !output || capacity < 32 || !isfinite(*input)) return 2;
    locale_t locale = newlocale(LC_NUMERIC_MASK, "C", (locale_t)0);
    if (!locale) return 3;
    locale_t previous = uselocale(locale);
    if (!previous) { freelocale(locale); return 3; }
    char buffer[32];
    int length = snprintf(buffer, sizeof buffer, "%.17g", *input);
    if (!uselocale(previous)) {
        /* Do not free a locale that might still be installed. */
        return 3;
    }
    freelocale(locale);
    if (length < 1 || length >= (int)sizeof buffer) return 2;
    memcpy(output, buffer, (size_t)length + 1);
    return 0;
}

int task096_abi(double input, double *positive, double *negative_zero) {
    if (task096_platform()) return 1;
    if (!positive || !negative_zero || input != 0x1.0000000000001p0) return 2;
    *positive = 0x1.0000000000001p0;
    *negative_zero = -0.0;
    return 0;
}