/* T1115: independent public C reader of real Rust lease publications. */
#define _GNU_SOURCE
#include "idle_control.h"
#include <ctype.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>

static int source_argument(int argc, char **argv)
{
    if (argc != 3)
        return -1;
    if (!isdigit((unsigned char)argv[2][0]))
        return -1;
    char *end;
    errno = 0;
    long source = strtol(argv[2], &end, 10);
    if (errno || *end || source < 3 || source > INT_MAX)
        return -1;
    if (fcntl((int)source, F_GETFD) < 0)
        return -1;
    return (int)source;
}

static int sample(const idle_control_reader_t *reader, const char *path, int source)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0)
        return 0;
    long long millis = (long long)now.tv_sec * 1000 + now.tv_nsec / 1000000;
    int interval = idle_control_reader_interval(reader, path, source, millis);
    if (printf("%d\n", interval) < 0)
        return 0;
    return fflush(stdout) == 0;
}

static int exchange(const idle_control_reader_t *reader, const char *path, int source)
{
    if (fputs("READY\n", stdout) < 0 || fflush(stdout) != 0)
        return 0;
    for (unsigned step = 0; step < 3; step++)
    {
        unsigned char command;
        if (read(STDIN_FILENO, &command, 1) != 1 || command != '?')
            return 0;
        if (!sample(reader, path, source))
            return 0;
    }
    return 1;
}

int main(int argc, char **argv)
{
    int source = source_argument(argc, argv);
    if (source < 0)
        return 2;
    idle_control_reader_t reader = IDLE_CONTROL_READER_INITIALIZER;
    if (!idle_control_begin(&reader, argv[1]))
    {
        close(source);
        return 3;
    }
    int accepted = exchange(&reader, argv[1], source);
    idle_control_end(&reader);
    int closed = close(source) == 0;
    return accepted && closed ? 0 : 4;
}
