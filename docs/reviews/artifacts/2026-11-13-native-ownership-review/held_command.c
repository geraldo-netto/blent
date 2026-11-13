/* Isolated command fixture; publishes its PID and waits for an owned file. */
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main(void)
{
    FILE *pid = fopen(getenv("BLENT_REVIEW_CHILD"), "w");
    if (!pid)
    {
        return 1;
    }
    fprintf(pid, "%ld", (long)getpid());
    fclose(pid);
    while (access(getenv("BLENT_REVIEW_RELEASE"), F_OK) != 0)
    {
        usleep(1000);
    }
    return 0;
}
