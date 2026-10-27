/* T1282: inventory ownership and disappearing topology; no DRM device access. */
#define _GNU_SOURCE
#include "../../scripts/benchmarks/evdi_snapshot/pattern.h"
#include <fcntl.h>
#include <unistd.h>
static void fixture_guest(void)
{
}
#define guest_only fixture_guest
#define main snapshot_main
#include "../../scripts/benchmarks/evdi_snapshot/produce.c"
#undef main
#undef guest_only

static int scenario, queries, objects, descriptors;
static uint32_t connector_id = 1, crtc_id = 2;
static drmModeModeInfo mode = {.hdisplay = SW, .vdisplay = SH};

static int current_fault(void)
{
    if (scenario < 20)
        return scenario;
    return queries > 1 ? scenario - 20 : 0;
}

int __real_open(const char *, int, ...);
int __wrap_open(const char *path, int flags, ...)
{
    (void)flags;
    if (strcmp(path, "/dev/dri/card0"))
        return -1;
    int fd = __real_open("/dev/null", O_RDWR);
    assert(fd >= 0);
    descriptors++;
    return fd;
}

int __real_close(int);
int __wrap_close(int fd)
{
    descriptors--;
    return __real_close(fd);
}

drmVersionPtr drmGetVersion(int fd)
{
    (void)fd;
    static drmVersion version = {.name = "evdi"};
    return &version;
}

void drmFreeVersion(drmVersionPtr version)
{
    (void)version;
}

drmModeResPtr drmModeGetResources(int fd)
{
    (void)fd;
    queries++;
    int fault = current_fault();
    if (fault == 1)
        return NULL;
    drmModeResPtr resources = calloc(1, sizeof(*resources));
    assert(resources);
    objects++;
    resources->count_connectors = fault == 2 ? 0 : 1;
    resources->connectors = fault == 3 ? NULL : &connector_id;
    resources->count_crtcs = fault == 4 ? 0 : 1;
    resources->crtcs = fault == 5 ? NULL : &crtc_id;
    return resources;
}

drmModeConnectorPtr drmModeGetConnector(int fd, uint32_t id)
{
    (void)fd;
    assert(id == connector_id);
    int fault = current_fault();
    if (fault == 6)
        return NULL;
    drmModeConnectorPtr connector = calloc(1, sizeof(*connector));
    assert(connector);
    objects++;
    connector->connector_id = connector_id;
    connector->connection = fault == 9 ? DRM_MODE_DISCONNECTED : DRM_MODE_CONNECTED;
    connector->count_modes = fault == 7 ? 0 : 1;
    connector->modes = fault == 8 ? NULL : &mode;
    return connector;
}

void drmModeFreeResources(drmModeResPtr resources)
{
    if (resources)
        objects--;
    free(resources);
}

void drmModeFreeConnector(drmModeConnectorPtr connector)
{
    if (connector)
        objects--;
    free(connector);
}

int drmSetClientCap(int fd, uint64_t capability, uint64_t value)
{
    (void)capability;
    (void)value;
    /* Valid admission only. Stop before any display or buffer operation. */
    assert(scenario == 0 && queries == 2 && objects == 0);
    close(fd);
    assert(descriptors == 0);
    exit(77);
}

int main(int argc, char **argv)
{
    assert(argc == 3);
    scenario = atoi(argv[1]);
    int result;
    if (strcmp(argv[2], "main") == 0)
        result = snapshot_main();
    else
    {
        int fd = connected_card();
        result = fd < 0 ? 2 : 0;
        if (fd >= 0)
            close(fd);
    }
    assert(objects == 0 && descriptors == 0);
    return result;
}
