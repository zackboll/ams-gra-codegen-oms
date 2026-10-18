#include "ams_gra_float64.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
extern unsigned task095_x87(void);
extern unsigned task095_mxcsr(void);
extern int task095_rounding_save(void);
extern int task095_rounding_restore(void);
extern int task095_rounding_pair(unsigned, unsigned);
extern void task095_mode(unsigned);
static int saved;
#define CHECK(x) do { if (!(x)) { if (saved) (void)task095_rounding_restore(); fprintf(stderr,"FAIL %d\n",__LINE__); exit(1); } } while (0)
static uint64_t bits(double value) {
    uint64_t result; memcpy(&result, &value, sizeof result); return result;
}
int main(int argc, char **argv) {
    CHECK(argc == 4);
    unsigned a = (unsigned)atoi(argv[1]), b = (unsigned)atoi(argv[2]);
    unsigned f = (unsigned)atoi(argv[3]);
    unsigned original_x87 = task095_x87(), original_sse = task095_mxcsr();
    CHECK(!task095_rounding_save()); saved = 1;
    CHECK(!task095_rounding_pair(a, b)); task095_mode(f);
    unsigned before_x87 = task095_x87(), before_sse = task095_mxcsr();
    int status = a == 0 && b == 0 && f == 0 ? 0 : 1;
    double output = 42.0, input = 1.0, positive = 42.0, negative = 42.0;
    char text[32]; memset(text, 'X', sizeof text);
    CHECK(ams_gra_float64_platform() == status);
    CHECK(ams_gra_float64_parse("1.0000000000000002", 18, 19, &output) == status);
    CHECK(ams_gra_float64_format(&input, text, sizeof text) == status);
    CHECK(ams_gra_float64_special(1, &positive) == status);
    CHECK(ams_gra_float64_abi(0x1.0000000000001p0, &positive, &negative) == status);
    CHECK(task095_x87() == before_x87);
    CHECK((task095_mxcsr() & ~63u) == (before_sse & ~63u));
    if (status) {
        CHECK(bits(output) == UINT64_C(0x4045000000000000));
        CHECK(bits(positive) == UINT64_C(0x4045000000000000));
        CHECK(bits(negative) == UINT64_C(0x4045000000000000));
        for (size_t i = 0; i < sizeof text; ++i) CHECK(text[i] == 'X');
    } else {
        CHECK(bits(output) == UINT64_C(0x3ff0000000000001));
        CHECK(bits(negative) == UINT64_C(0x8000000000000000));
        CHECK(!strcmp(text, "1"));
    }
    CHECK(!task095_rounding_restore()); saved = 0;
    CHECK(task095_x87() == original_x87 && task095_mxcsr() == original_sse);
    CHECK(!ams_gra_float64_parse("-0", 2, 3, &output));
    CHECK(bits(output) == UINT64_C(0x8000000000000000));
    CHECK(!ams_gra_float64_format(&output, text, sizeof text));
    CHECK(!strcmp(text, "-0"));
    printf("TASK097_C_ENVIRONMENT_COMPLETE x87=%u mxcsr=%u flushing=%u\n", a,b,f);
    return 0;
}