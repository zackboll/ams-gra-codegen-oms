#ifndef AMS_GRA_FLOAT64_H
#define AMS_GRA_FLOAT64_H
#include <stddef.h>
/* 0 success; 1 unsupported platform/control state; 2 invalid value;
   3 resource/formatter failure. Outputs are untouched on returned failure.
   A failed thread-locale restoration is NOT recoverable: the process aborts
   before publication, without freeing a possibly installed locale. */
int ams_gra_float64_platform(void);
int ams_gra_float64_parse(const char *, size_t, size_t, double *);
int ams_gra_float64_format(const double *, char *, size_t);
int ams_gra_float64_special(int, double *);
int ams_gra_float64_abi(double, double *, double *);
#endif