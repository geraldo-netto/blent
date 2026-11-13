/* Fault injection applies only to the exact private fixture child/group. */
#define _GNU_SOURCE
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/syscall.h>
#include <unistd.h>

static long original(void)
{
    const char *path = getenv("BLENT_REVIEW_CHILD");
    FILE *file = path ? fopen(path, "r") : NULL;
    long pid = 0;
    if (file)
    {
        if (fscanf(file, "%ld", &pid) != 1)
        {
            pid = 0;
        }
        fclose(file);
    }
    return pid;
}

int kill(pid_t pid, int signal)
{
    long owned = original();
    if (owned > 0 && signal == SIGKILL && (pid == owned || pid == -owned))
    {
        errno = EPERM;
        return -1;
    }
    return syscall(SYS_kill, pid, signal);
}
