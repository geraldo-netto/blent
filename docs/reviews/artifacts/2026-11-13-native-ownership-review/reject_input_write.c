/* Transient write failure affects only the private fake uinput descriptor. */
#define _GNU_SOURCE
#include <errno.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

static int rejected(int fd)
{
    const char *device = getenv("BLENT_T497_UINPUT_FILE");
    const char *gate = getenv("BLENT_REVIEW_WRITE_FAIL");
    struct stat actual, expected;
    return device && gate && access(gate, F_OK) == 0 &&
        stat(device, &expected) == 0 && fstat(fd, &actual) == 0 &&
        actual.st_dev == expected.st_dev && actual.st_ino == expected.st_ino;
}

ssize_t write(int fd, const void *bytes, size_t count)
{
    if (rejected(fd))
    {
        errno = EIO;
        return -1;
    }
    return syscall(SYS_write, fd, bytes, count);
}
