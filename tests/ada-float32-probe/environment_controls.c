#include <fenv.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <xmmintrin.h>
extern int ams_gra_float32_platform(void);
extern int ams_gra_float32_abi(float, float *, float *);
extern int ams_gra_float32_parse(const char *, size_t, size_t, float *);
extern int ams_gra_float32_special(int, float *);
extern double task095_widen(float);
enum { DAZ = 1u << 6, FTZ = 1u << 15, CONTROLS = 0xffc0u };
static unsigned checks;
static uint32_t bits(float x) { uint32_t b; memcpy(&b,&x,4); return b; }
static uint64_t wide_bits(double x) { uint64_t b; memcpy(&b,&x,8); return b; }
static int check(int condition, unsigned line) {
    ++checks;
    if (!condition) fprintf(stderr,"failure line %u\n",line);
    return condition;
}
#define CHECK(x) do { if (!check((x),__LINE__)) goto failure; } while (0)
int main(int argc, char **argv) {
    const unsigned saved = _mm_getcsr();
    int success = 0;
    if (argc < 2) goto failure;
    const unsigned mode = (unsigned)strtoul(argv[1],NULL,10);
    const int red = argc > 2;
    if (mode > 3) goto failure;
    unsigned selected = (saved & ~(DAZ|FTZ)) | ((mode & 1) ? DAZ : 0) | ((mode & 2) ? FTZ : 0);
    struct vector { const char *text; uint32_t want; } vectors[] = {
        {"1.40129846432481707092372958328991613e-45",1},
        {"-1.40129846432481707092372958328991613e-45",0x80000001},
        {"1e-40",0x116c2},{"-1e-40",0x800116c2},
        {"1.17549421069244107548702944484928735e-38",0x7fffff},
        {"1.17549435082228750796873653722224568e-38",0x800000},
        {"0",0},{"-0",0x80000000}
    };
    _mm_setcsr(saved & ~(DAZ|FTZ));
    CHECK(ams_gra_float32_platform()==0);
    for (size_t i=0;i<sizeof vectors/sizeof vectors[0];++i) {
        float output, positive, negative;
        const uint32_t sentinel=0x42f60000;
        memcpy(&output,&sentinel,4);memcpy(&positive,&sentinel,4);memcpy(&negative,&sentinel,4);
        _mm_setcsr(selected);
        unsigned before=_mm_getcsr();
        int platform=ams_gra_float32_platform();
        int abi=ams_gra_float32_abi(0x1.000002p0f,&positive,&negative);
        int parsed=ams_gra_float32_parse(vectors[i].text,strlen(vectors[i].text),strlen(vectors[i].text)+1,&output);
        unsigned after=_mm_getcsr(); /* measure before logging or widening */
        uint32_t raw=bits(output);
        double widened=task095_widen(output);
        uint64_t wide=wide_bits(widened);
        _mm_setcsr(saved & ~(DAZ|FTZ));
        if (red) {
            printf("OBSERVE mode=%u input=%s platform=%d abi=%d parse=%d raw=%08x wide=%016llx controls=%08x/%08x\n",mode,vectors[i].text,platform,abi,parsed,raw,(unsigned long long)wide,before&CONTROLS,after&CONTROLS);
        } else {
            CHECK((before&CONTROLS)==(after&CONTROLS));
            if (mode==0) {
                CHECK(platform==0 && abi==0 && parsed==0);
                CHECK(raw==vectors[i].want);
            } else {
                CHECK(platform==1 && abi==1 && parsed==1);
                CHECK(raw==sentinel && bits(positive)==sentinel && bits(negative)==sentinel);
            }
        }
    }
    for (int which=-1;which<=1;++which) {
        float output=123.0f;
        _mm_setcsr(selected);unsigned before=_mm_getcsr();
        int status=ams_gra_float32_special(which,&output);
        unsigned after=_mm_getcsr();
        _mm_setcsr(saved & ~(DAZ|FTZ));
        if (!red) {
            CHECK((before&CONTROLS)==(after&CONTROLS));
            CHECK(status==(mode ? 1 : 0));
            if (mode) CHECK(bits(output)==0x42f60000);
        }
    }
    _mm_setcsr(saved & ~(DAZ|FTZ));
    CHECK(ams_gra_float32_platform()==0); /* revalidate after restoration */
    if (!red && checks!=(mode ? 35u : 32u)) goto failure;
    success=1;
failure:
    _mm_setcsr(saved);
    if (success) printf("TASK095_C2_C_ENVIRONMENT_COMPLETE mode=%s checks=%u\n",argv[1],checks);
    return success ? 0 : 1;
}