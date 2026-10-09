/* Shared guest precondition and EVDI ownership discovery for native probes. */
#ifndef BLENT_REVIEW_PROBE_DEVICE_H
#define BLENT_REVIEW_PROBE_DEVICE_H
static void probe_require_guest(void) {
    char command[4096] = {0};
    FILE *file = fopen("/proc/cmdline", "r");
    if (!file) exit(64);
    int read = fgets(command, sizeof(command), file) != NULL;
    fclose(file);
    if (!read || !strstr(command, "blent-evdi-review=1")) exit(64);
}

static int probe_open_evdi(int *card, drmVersionPtr *version) {
    for (*card = 0; *card < 8; (*card)++) {
        char path[64];
        snprintf(path, sizeof(path), "/dev/dri/card%d", *card);
        int fd = open(path, O_RDWR);
        *version = drmGetVersion(fd);
        if (*version && !strcmp((*version)->name, "evdi")) return fd;
        if (*version) drmFreeVersion(*version);
        *version = NULL;
        if (fd >= 0) close(fd);
    }
    exit(65);
}
#endif
