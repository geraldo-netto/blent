/* T1439 private empty connector inventory; never consult the live desktop. */
#define _GNU_SOURCE
#include <dirent.h>
#include <dlfcn.h>
#include <stdlib.h>
#include <string.h>

DIR *opendir(const char *path)
{
    DIR *(*original)(const char *) = dlsym(RTLD_NEXT, "opendir");
    if (strcmp(path, "/sys/class/drm") == 0 || strcmp(path, "/sys/devices/platform") == 0)
    {
        return original(getenv("BLENT_T1439_SYSFS"));
    }
    return original(path);
}
