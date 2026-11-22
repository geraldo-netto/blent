#define _POSIX_C_SOURCE 200809L
#include "accessory.h"
#include <stdio.h>
#include <assert.h>
#include <dirent.h>
#include <unistd.h>

static int descriptors(void)
{
    DIR *directory = opendir("/proc/self/fd");
    assert(directory);
    int count = 0;
    while (readdir(directory))
    {
        count++;
    }
    closedir(directory);
    return count;
}

int main(int argc, char **argv)
{
    if (argc != 3)
    {
        return 9;
    }
    accessory_target target = accessory_parse_target(argv[1], argv[2]);
    int before = descriptors();
    int lock = accessory_lock(target);
    assert(descriptors() == before + 1);
    puts("locked");
    fflush(stdout);
    char byte;
    (void)read(STDIN_FILENO, &byte, 1);
    close(lock);
    assert(descriptors() == before);
    return 0;
}
