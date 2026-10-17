#include <fenv.h>
#include <xmmintrin.h>
/* Test-only raw controls: Intel SDM Vol. 1, x87 control-word RC bits
   10..11 and MXCSR RC bits 13..14. FE_* encodings are not MXCSR encodings. */
enum { X87_RC_MASK = 3u << 10 };
static fenv_t saved_environment;
static unsigned saved_mxcsr;
static int saved;
unsigned task095_x87(void) {
    unsigned short word;
    __asm__ volatile ("fnstcw %0" : "=m" (word));
    return word;
}
unsigned task095_mxcsr(void) { return _mm_getcsr(); }
int task095_rounding_save(void) {
    if (saved || fegetenv(&saved_environment)) return 1;
    saved_mxcsr = _mm_getcsr();
    saved = 1;
    return 0;
}
int task095_rounding_restore(void) {
    if (!saved || fesetenv(&saved_environment)) return 1;
    /* Explicit MXCSR restoration retains an independently mismatched pair. */
    _mm_setcsr(saved_mxcsr);
    saved = 0;
    return 0;
}
int task095_rounding_pair(unsigned x87, unsigned sse) {
    static const unsigned x87_bits[] = {0u, 1u << 10, 2u << 10, 3u << 10};
    static const unsigned sse_bits[] = {_MM_ROUND_NEAREST, _MM_ROUND_DOWN,
                                      _MM_ROUND_UP, _MM_ROUND_TOWARD_ZERO};
    if (x87 > 3 || sse > 3) return 1;
    unsigned short word = (unsigned short)((task095_x87() & ~X87_RC_MASK) | x87_bits[x87]);
    const unsigned mxcsr = (_mm_getcsr() & ~_MM_ROUND_MASK) | sse_bits[sse];
    __asm__ volatile ("fldcw %0" : : "m" (word));
    _mm_setcsr(mxcsr);
    return (task095_x87() & X87_RC_MASK) != x87_bits[x87] ||
           (_mm_getcsr() & _MM_ROUND_MASK) != sse_bits[sse];
}
int task095_rounding_fe(unsigned index) {
    static const int modes[] = {FE_TONEAREST, FE_DOWNWARD, FE_UPWARD, FE_TOWARDZERO};
    return index < 4 ? modes[index] : -1;
}