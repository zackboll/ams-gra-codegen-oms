/* O0/O2 independent C campaigns; Python exact oracle assesses emitted tokens. */
#define _GNU_SOURCE
#include <errno.h>
#include <inttypes.h>
#include <locale.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
extern int task096_parse(const char *, size_t, size_t, double *, size_t *, int *);
extern int task096_format(const double *, char *, size_t);
int main(int argc, char **argv) {
    if (argc != 4) return 1;
    locale_t locale = newlocale(LC_NUMERIC_MASK, argv[3], (locale_t)0);
    if (!locale) return 2;
    locale_t previous = uselocale(locale);
    if (!previous) return 3;
    FILE *input = fopen(argv[2], "r");
    if (!input) return 4;
    char *line = NULL;
    size_t capacity = 0, count = 0;
    ssize_t length;
    int failed = 0;
    while ((length = getline(&line, &capacity, input)) > 0) {
        if (line[length - 1] == '\n') line[--length] = 0;
        count++;
        if (!strcmp(argv[1], "convert")) {
            char *expected = strchr(line, '\t');
            if (!expected) { failed = 1; break; }
            *expected++ = 0;
            char *token = strchr(expected, '\t');
            if (!token) { failed = 1; break; }
            *token++ = 0;
            double value = 42.0;
            size_t used = 0;
            int error = 0;
            int status = task096_parse(token, strlen(token), strlen(token) + 1, &value, &used, &error);
            uint64_t bits;
            memcpy(&bits, &value, sizeof bits);
            char actual[17];
            snprintf(actual, sizeof actual, "%016" PRIX64, bits);
            int valid = !strcmp(expected, "OVERFLOW") ? status == 2 && bits == UINT64_C(0x4045000000000000)
                : status == 0 && !strcmp(expected, actual) && used == strlen(token) && (error == 0 || error == ERANGE);
            if (!valid) { printf("MISMATCH\t%s\t%s\t%d\t%s\n",expected,actual,status,token); failed = 1; }
        } else if (!strcmp(argv[1], "encode")) {
            uint64_t bits = strtoull(line, NULL, 16);
            double value;
            memcpy(&value, &bits, sizeof value);
            char output[32];
            if (task096_format(&value, output, sizeof output)) { failed = 1; break; }
            printf("format17\t%s\t%s\n",line,output);
        } else { failed = 1; break; }
        if (uselocale((locale_t)0) != locale) { failed = 1; break; }
        if (count % 2048 == 0) fflush(stdout);
    }
    free(line);
    fclose(input);
    if (uselocale(previous) != locale) failed = 1;
    freelocale(locale);
    if (uselocale((locale_t)0) != previous) failed = 1;
    printf("TASK096_C_CORPUS_COMPLETE mode=%s count=%zu failures=%d\n",argv[1],count,failed);
    return failed;
}