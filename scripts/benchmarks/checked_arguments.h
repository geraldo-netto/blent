#ifndef BLENT_BENCHMARK_CHECKED_ARGUMENTS_H
#define BLENT_BENCHMARK_CHECKED_ARGUMENTS_H
/* These diagnostic executables deliberately execute checked calls in assert.
 * Reject builds that would remove those calls instead of silently misrunning. */
#ifdef NDEBUG
#error "Blent benchmark executables require assertions; do not define NDEBUG"
#endif
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

static inline int benchmark_number(const char *text, int minimum, int maximum) {
    char *end;
    errno = 0;
    long number = strtol(text, &end, 10);
    if (errno || end == text || *end || number < minimum || number > maximum) {
        fprintf(stderr, "Invalid benchmark integer: %s\n", text);
        exit(2);
    }
    return (int)number;
}
#endif
