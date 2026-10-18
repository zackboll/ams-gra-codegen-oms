/* Test-only ambient locale control. Never changes the process-global locale. */
#define _GNU_SOURCE
#include <locale.h>
static locale_t previous, installed;
int task096_locale_enter(void) {
    if (installed) return 1;
    installed = newlocale(LC_NUMERIC_MASK, "de_DE.UTF-8", (locale_t)0);
    if (!installed) return 1;
    previous = uselocale(installed);
    if (!previous) { freelocale(installed); installed = (locale_t)0; return 1; }
    return 0;
}
int task096_locale_leave(void) {
    if (!installed || uselocale((locale_t)0) != installed) return 1;
    if (uselocale(previous) != installed) return 1;
    freelocale(installed);
    installed = (locale_t)0;
    return uselocale((locale_t)0) == previous ? 0 : 1;
}