#include <fenv.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <xmmintrin.h>
extern int task095_rounding_save(void), task095_rounding_restore(void);
extern int task095_rounding_pair(unsigned, unsigned), task095_rounding_fe(unsigned);
extern unsigned task095_x87(void), task095_mxcsr(void);
extern int ams_gra_float32_platform(void);
extern int ams_gra_float32_abi(float, float *, float *);
extern int ams_gra_float32_parse(const char *, size_t, size_t, float *);
extern int ams_gra_float32_special(int, float *);
static unsigned checks;
static int saved;
#define CHECK(x) do { ++checks; if (!(x)) { fprintf(stderr,"failure line %d\n",__LINE__); goto failure; } } while (0)
static uint32_t bits(float x) { uint32_t result; memcpy(&result,&x,4); return result; }
int main(int argc, char **argv) {
    if (argc < 3) return 1;
    const unsigned x = (unsigned)strtoul(argv[1],NULL,10);
    const unsigned s = (unsigned)strtoul(argv[2],NULL,10);
    const int red = argc > 3;
    const unsigned original_x = task095_x87(), original_s = task095_mxcsr();
    CHECK(task095_rounding_save()==0); saved=1;
    CHECK(task095_rounding_pair(0,0)==0);
    CHECK((task095_mxcsr() & ((1u<<6)|(1u<<15)))==0);
    CHECK(ams_gra_float32_platform()==0);
    const char *tokens[] = {"1.000000059604644775390624", "1.000000059604644775390626",
                           "1.000000178813934326171874", "-1.000000059604644775390626", "0", "-0", "-0.0"};
    const uint32_t expected[] = {0x3f800000,0x3f800001,0x3f800001,0xbf800001,0,0x80000000,0x80000000};
    for (unsigned i=0;i<7;++i) {
        float output=123.0f, positive=123.0f, negative=123.0f, special=123.0f;
        CHECK(task095_rounding_pair(x,s)==0);
        unsigned before_x=task095_x87(), before_s=task095_mxcsr();
        int fe=fegetround();
        CHECK(fe==task095_rounding_fe(x));
        int platform=ams_gra_float32_platform();
        int abi=ams_gra_float32_abi(0x1.000002p0f,&positive,&negative);
        int parse=ams_gra_float32_parse(tokens[i],strlen(tokens[i]),strlen(tokens[i])+1,&output);
        int sp=ams_gra_float32_special(1,&special);
        unsigned after_x=task095_x87(), after_s=task095_mxcsr();
        uint32_t raw=bits(output), pos=bits(positive), neg=bits(negative), sb=bits(special);
        CHECK(before_x==after_x && (before_s&0xffc0u)==(after_s&0xffc0u));
        CHECK(task095_rounding_pair(0,0)==0);
        printf("PAIR x87=%u mxcsr=%u token=%u fe=%d raw_x87=%04x raw_mxcsr=%08x after_x87=%04x after_mxcsr=%08x platform=%d abi=%d parse=%d special=%d bits=%08x\n",x,s,i,fe,before_x,before_s,after_x,after_s,platform,abi,parse,sp,raw);
        if (!red) {
            const int want=(x||s)?1:0;
            CHECK(platform==want && abi==want && parse==want && sp==want);
            if (want) CHECK(raw==0x42f60000 && pos==0x42f60000 && neg==0x42f60000 && sb==0x42f60000);
            else CHECK(raw==expected[i] && pos==0x3f800001 && neg==0x80000000 && sb==0x7f800000);
        }
    }
    CHECK(ams_gra_float32_platform()==0);
    CHECK(task095_rounding_restore()==0);saved=0;
    CHECK(task095_x87()==original_x && task095_mxcsr()==original_s);
    /* Verify full restore of an initially mismatched pair, not fesetround only. */
    CHECK(task095_rounding_save()==0);saved=1;
    CHECK(task095_rounding_pair(1,2)==0);
    unsigned mismatch_x=task095_x87(), mismatch_s=task095_mxcsr();
    fenv_t mismatch; CHECK(fegetenv(&mismatch)==0);
    CHECK(task095_rounding_pair(0,0)==0);
    CHECK(fesetenv(&mismatch)==0); _mm_setcsr(mismatch_s);
    CHECK(task095_x87()==mismatch_x && task095_mxcsr()==mismatch_s);
    CHECK(task095_rounding_restore()==0);saved=0;
    CHECK(task095_x87()==original_x && task095_mxcsr()==original_s);
    const unsigned want_checks=red?43:57;
    if (checks!=want_checks) {fprintf(stderr,"count=%u want=%u\n",checks,want_checks);return 1;}
    printf("TASK095_C3_C_ROUNDING_COMPLETE x87=%u mxcsr=%u checks=%u red=%d\n",x,s,checks,red);
    return 0;
failure:
    if (saved) (void)task095_rounding_restore();
    return 1;
}