#define _GNU_SOURCE
#include "idle_control.h"
#include <ctype.h>
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <unistd.h>

static int advice_current(int rejected_fd, const struct stat *current)
{
    if (rejected_fd < 0)
        return 1;
    struct stat original;
    if (fstat(rejected_fd, &original) != 0)
        return 0;
    return original.st_dev != current->st_dev || original.st_ino != current->st_ino;
}

static ssize_t read_idle_control(const char *path, char *text, size_t capacity, int rejected_fd)
{
    if (!path)
        return -1;
    int file = open(path, O_RDONLY | O_NONBLOCK | O_NOFOLLOW | O_CLOEXEC);
    if (file < 0)
        return -1;
    struct stat info;
    ssize_t count = -1;
    if (fstat(file, &info) == 0 && S_ISREG(info.st_mode) && info.st_uid == geteuid() &&
        !(info.st_mode & (S_IWGRP | S_IWOTH)) && advice_current(rejected_fd, &info))
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

static int read_interval(const char *path, int source, long long now_ms, int rejected_fd)
{
    char text[160];
    ssize_t count = read_idle_control(path, text, sizeof(text), rejected_fd);
    return idle_control_value(text, count, source, now_ms);
}

int idle_control_interval(const char *path, int source, long long now_ms)
{
    return read_interval(path, source, now_ms, -1);
}

static int initial_advice_valid(int file)
{
    struct stat info;
    return fstat(file, &info) == 0 && S_ISREG(info.st_mode) && info.st_uid == geteuid() &&
           !(info.st_mode & (S_IWGRP | S_IWOTH)) && info.st_size >= 0 && info.st_size < 160;
}

int idle_control_begin(idle_control_reader_t *reader, const char *path)
{
    if (reader->admitted)
        return 0;
    reader->fd = -1;
    if (!path)
    {
        reader->admitted = 1;
        return 1;
    }
    int file = open(path, O_PATH | O_NOFOLLOW | O_CLOEXEC);
    if (file < 0)
    {
        if (errno == ENOENT)
            reader->admitted = 1;
        return reader->admitted;
    }
    if (!initial_advice_valid(file))
    {
        close(file);
        return 0;
    }
    reader->fd = file;
    reader->admitted = 1;
    return 1;
}

int idle_control_reader_interval(const idle_control_reader_t *reader, const char *path, int source,
                                 long long now_ms)
{
    if (!reader->admitted)
        return 0;
    return read_interval(path, source, now_ms, reader->fd);
}

void idle_control_end(idle_control_reader_t *reader)
{
    if (reader->admitted && reader->fd >= 0)
        close(reader->fd);
    reader->fd = -1;
    reader->admitted = 0;
}
