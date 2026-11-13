/* T1433 private fake busctl: retain only the first exact owned command. */
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static void record_call(void)
{
    FILE *file = fopen(getenv("BLENT_T1433_CALLS"), "a");
    if (!file)
    {
        abort();
    }
    fprintf(file, "%ld\n", (long)getpid());
    fclose(file);
}

static void hold_original(void)
{
    int fd = open(getenv("BLENT_T1433_CHILD"), O_WRONLY | O_CREAT | O_EXCL, 0600);
    if (fd < 0)
    {
        return;
    }
    dprintf(fd, "%ld", (long)getpid());
    close(fd);
    while (access(getenv("BLENT_T1433_RELEASE"), F_OK) != 0)
    {
        usleep(1000);
    }
}

int main(int argc, char **argv)
{
    record_call();
    hold_original();
    if (argc > 1 && strcmp(argv[argc - 1], "available") == 0)
    {
        puts("b true");
    }
    else
    {
        puts("i 1");
    }
    return 0;
}
