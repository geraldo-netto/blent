/* T1117: production C accepts numerically identical padded source cookies. */
#define _GNU_SOURCE
#include "writer.c"
#include <assert.h>

int main(void)
{
    int ends[2];
    assert(pipe(ends) == 0);
    struct stat identity;
    assert(fstat(ends[1], &identity) == 0);
    const int widths[] = {0, 1, 16, 20};
    for (unsigned index = 0; index < sizeof(widths) / sizeof(widths[0]); index++)
    {
        char text[160];
        int width = widths[index];
        int count = snprintf(text, sizeof(text), "%0*ju %0*ju %0*ju %0*ju %0*ju\n",
                             width, (uintmax_t)identity.st_dev, width, (uintmax_t)identity.st_ino,
                             width, (uintmax_t)1, width, (uintmax_t)1200, width, (uintmax_t)500);
        assert(count > 0);
        assert(count < (int)sizeof(text));
        assert(idle_control_value(text, count, ends[1], 1000) == 500);
    }
    assert(close(ends[0]) == 0);
    assert(close(ends[1]) == 0);
    return 0;
}
