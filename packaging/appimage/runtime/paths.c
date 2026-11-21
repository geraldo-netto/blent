#define _GNU_SOURCE
#include "paths.h"
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int blent_path(char *output, size_t capacity, const char *base, const char *suffix)
{
    size_t first = strnlen(base, capacity);
    if (first >= capacity)
    {
        errno = ENAMETOOLONG;
        return -1;
    }
    size_t second = strnlen(suffix, capacity - first);
    if (second >= capacity - first)
    {
        errno = ENAMETOOLONG;
        return -1;
    }
    memmove(output, base, first);
    memmove(output + first, suffix, second + 1);
    return 0;
}

void blent_path_required(char *output, size_t capacity, const char *base, const char *suffix)
{
    if (blent_path(output, capacity, base, suffix) < 0)
    {
        perror("AppImage path");
        exit(127);
    }
}

ssize_t blent_readlink(const char *path, char *output, size_t capacity)
{
    ssize_t length = readlink(path, output, capacity);
    if (length < 0)
    {
        return -1;
    }
    if ((size_t)length >= capacity)
    {
        errno = ENAMETOOLONG;
        return -1;
    }
    output[length] = '\0';
    return length;
}
