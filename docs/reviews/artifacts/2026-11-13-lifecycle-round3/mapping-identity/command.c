/* T1439: private X11/KWin commands; only the original mapper can remain held. */
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static void record_call(const char *tool, int argc, char **argv)
{
    FILE *file = fopen(getenv("BLENT_T1439_CALLS"), "a");
    if (!file)
    {
        abort();
    }
    fprintf(file, "%s", tool);
    for (int i = 1; i < argc; i++)
    {
        fprintf(file, " %s", argv[i]);
    }
    fprintf(file, "\n");
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
    while (access(getenv("BLENT_T1439_RELEASE"), F_OK) != 0)
    {
        usleep(1000);
    }
}

static int visible_id(void)
{
    char path[4096];
    snprintf(path, sizeof(path), "%s.new", getenv("BLENT_T1439_RELEASE"));
    return access(path, F_OK) == 0 ? 42 : 41;
}

static int xinput(int argc, char **argv)
{
    if (argc > 1 && strcmp(argv[1], "map-to-output") == 0)
    {
        hold_original();
        const char *status = getenv("BLENT_T1439_MAP_STATUS");
        return status ? atoi(status) : 0;
    }
    else if (argc > 1 && strcmp(argv[1], "list-props") == 0)
    {
        printf("Device Node (12): \"/dev/input/event%d\"\nCoordinate Transformation Matrix (13): 1,0,0,0,1,0,0,0,1\n", visible_id());
    }
    else
    {
        printf("Blent Touch id=%d [slave pointer]\n", visible_id());
    }
    return 0;
}

static void randr_output(void)
{
    unsigned char edid[128] = {0, 255, 255, 255, 255, 255, 255, 0};
    edid[127] = 6;
    puts("HDMI-1 connected primary 1920x1080+0+0 normal\n\tEDID:");
    for (int row = 0; row < 8; row++)
    {
        printf("\t\t");
        for (int column = 0; column < 16; column++)
        {
            printf("%02x", edid[row * 16 + column]);
        }
        printf("\n");
    }
}

int main(int argc, char **argv)
{
    const char *tool = strrchr(argv[0], '/');
    tool = tool ? tool + 1 : argv[0];
    record_call(tool, argc, argv);
    if (strcmp(tool, "xrandr") == 0)
    {
        randr_output();
    }
    else if (strcmp(tool, "xinput") == 0)
    {
        return xinput(argc, argv);
    }
    else if (argc > 1 && strcmp(argv[argc - 1], "available") == 0)
    {
        puts("b true");
    }
    else
    {
        puts("i 1");
    }
    return 0;
}
