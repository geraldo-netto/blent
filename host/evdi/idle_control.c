#define _GNU_SOURCE
#include "idle_control.h"
#include <ctype.h>
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <unistd.h>

static ssize_t read_idle_control(const char *path, char *text, size_t capacity)
{
    if (!path)
        return -1;
    int file = open(path, O_RDONLY | O_NONBLOCK | O_NOFOLLOW | O_CLOEXEC);
    if (file < 0)
        return -1;
    struct stat info;
    ssize_t count = -1;
    if (fstat(file, &info) == 0 && S_ISREG(info.st_mode) && info.st_uid == geteuid() &&
        !(info.st_mode & (S_IWGRP | S_IWOTH)))
        count = read(file, text, capacity);
    close(file);
    return count;
}

/* Five unsigned fields; return borrowed end cursor for exact-length validation. */
static const char *idle_fields(const char *text, uintmax_t fields[5])
{
    for (int index = 0; index < 5; index++)
    {
        if (!isdigit((unsigned char)*text))
            return NULL;
        char *end;
        errno = 0;
        fields[index] = strtoumax(text, &end, 10);
        if (errno == ERANGE || (*end != ' ' && *end != '\n'))
            return NULL;
        text = end + 1;
    }
    return text;
}

static int idle_source_matches(int source, const uintmax_t fields[5])
{
    struct stat info;
    return fstat(source, &info) == 0 && fields[0] == info.st_dev && fields[1] == info.st_ino;
}

static int idle_control_value(char *text, ssize_t count, int source, long long now)
{
    if (count < 1 || count >= 160)
        return 0;
    text[count] = '\0';
    uintmax_t fields[5];
    if (idle_fields(text, fields) != text + count || (fields[4] != 500 && fields[4] != 1000))
        return 0;
    if (fields[3] <= (uintmax_t)now || fields[3] - (uintmax_t)now > 2000)
        return 0;
    if (!idle_source_matches(source, fields))
        return 0;
    return (int)fields[4];
}

int idle_control_interval(const char *path, int source, long long now_ms)
{
    char text[160];
    ssize_t count = read_idle_control(path, text, sizeof(text));
    return idle_control_value(text, count, source, now_ms);
}
