#include <xmmintrin.h>
#include <stdint.h>
#include <string.h>
/* Intel SDM Vol 1, MXCSR: DAZ bit 6, FTZ bit 15. Test-only mutation;
   production may inspect but must never modify these controls. */
enum { DAZ = 1u << 6, FTZ = 1u << 15 };
unsigned task095_controls(void) { return _mm_getcsr(); }
void task095_restore(unsigned saved) { _mm_setcsr(saved); }
void task095_mode(unsigned mode) {
    unsigned value = _mm_getcsr() & ~(DAZ | FTZ);
    if (mode & 1) value |= DAZ;
    if (mode & 2) value |= FTZ;
    _mm_setcsr(value);
}
unsigned task095_bits(float value) {
    uint32_t bits;
    memcpy(&bits, &value, sizeof bits);
    return bits;
}
float task095_from_bits(unsigned bits) {
    float value;
    memcpy(&value, &bits, sizeof value);
    return value;
}
double task095_widen(float value) { return (double)value; }