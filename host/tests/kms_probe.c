/* T765: guest probe failure paths use fake native interfaces only. */
#define _GNU_SOURCE
#include <xf86drm.h>
#include <xf86drmMode.h>
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <poll.h>
#include <unistd.h>
#include <stdint.h>
#include "evdi_lib.h"
static int scenario, live_objects, live_buffers, live_device, registered;
static void probe_require_guest(void) {}
static int probe_open_evdi(int *card, drmVersionPtr *version) {
    *card = 0; *version = calloc(1, sizeof(**version)); return 99;
}
static FILE *test_fopen(const char *path, const char *mode) {
    (void)path; (void)mode;
    FILE *file = tmpfile(); assert(file);
    unsigned char bytes[128] = {0}; assert(fwrite(bytes, 1, 128, file) == 128); rewind(file);
    return file;
}
static void *test_calloc(size_t count, size_t size) {
    if (scenario == 1 && count == 640 * 480) return NULL;
    return calloc(count, size);
}
static void *test_mmap(void *, size_t, int, int, int, off_t);
static int test_munmap(void *, size_t);
static int test_poll(struct pollfd *fds, nfds_t count, int timeout) {
    (void)fds; (void)count; (void)timeout; return 0;
}
#define mmap test_mmap
#define munmap test_munmap
#define poll test_poll
#define BLENT_REVIEW_PROBE_DEVICE_H
#define fopen test_fopen
#define calloc test_calloc
#define main probe_main
#include "../../docs/reviews/artifacts/2026-10-09-evdi-stock/kms-probe.c"
#undef main
#undef calloc
#undef fopen
#undef mmap
#undef munmap
#undef poll

void drmFreeVersion(drmVersionPtr value) { free(value); }
drmModeResPtr drmModeGetResources(int fd) {
    (void)fd;
    static uint32_t crtc = 1, connector = 2;
    drmModeResPtr r = calloc(1, sizeof(*r)); live_objects++;
    r->count_connectors = r->count_crtcs = 1; r->connectors = &connector; r->crtcs = &crtc;
    return r;
}
drmModeConnectorPtr drmModeGetConnector(int fd, uint32_t id) {
    (void)fd; (void)id;
    static drmModeModeInfo mode = {.hdisplay=640, .vdisplay=480};
    drmModeConnectorPtr c = calloc(1, sizeof(*c)); live_objects++; c->count_modes=1; c->modes=&mode; return c;
}
int drmIoctl(int fd, unsigned long request, void *value) {
    (void)fd;
    if (request == DRM_IOCTL_MODE_CREATE_DUMB) {
        struct drm_mode_create_dumb *b=value; b->handle=(uint32_t)++live_buffers; b->pitch=b->width*4; b->size=(uint64_t)b->pitch*b->height;
    }
    if (request == DRM_IOCTL_MODE_DESTROY_DUMB) live_buffers--;
    return 0;
}
static void *test_mmap(void *address, size_t size, int protection, int flags, int fd, off_t offset) {
    (void)address; (void)protection; (void)flags; (void)fd; (void)offset; return malloc(size);
}
static int test_munmap(void *address, size_t size) { (void)size; free(address); return 0; }
int drmModeAddFB2(int fd, uint32_t w, uint32_t h, uint32_t format, const uint32_t handles[4], const uint32_t pitches[4], const uint32_t offsets[4], uint32_t *id, uint32_t flags) {
    (void)fd; (void)w; (void)h; (void)format; (void)handles; (void)pitches; (void)offsets; (void)flags; if (scenario == 5 && w == 64) return -1; *id=1; return 0;
}
int drmModeSetCrtc(int fd, uint32_t crtc, uint32_t fb, uint32_t x, uint32_t y, uint32_t *connectors, int count, drmModeModeInfoPtr mode) {
    (void)fd; (void)crtc; (void)fb; (void)x; (void)y; (void)connectors; (void)count; (void)mode; return 0;
}
int drmSetClientCap(int fd, uint64_t cap, uint64_t value) { (void)fd; (void)cap; (void)value; return 0; }
drmModePlaneResPtr drmModeGetPlaneResources(int fd) {
    (void)fd; static uint32_t planes[2]={41,42};
    if (scenario == 2) return NULL;
    drmModePlaneResPtr p=calloc(1,sizeof(*p)); live_objects++;
    p->count_planes=scenario == 3 ? 0 : 2; p->planes=planes; return p;
}
int drmModeSetPlane(int fd, uint32_t plane, uint32_t crtc, uint32_t fb, uint32_t flags, int32_t x, int32_t y, uint32_t w, uint32_t h, uint32_t sx, uint32_t sy, uint32_t sw, uint32_t sh) {
    (void)fd; (void)plane; (void)crtc; (void)fb; (void)flags; (void)x; (void)y; (void)w; (void)h; (void)sx; (void)sy; (void)sw; (void)sh; assert(plane == 41);
    return (scenario == 6 && fb) || (scenario == 7 && !fb) ? -1 : 0;
}
evdi_handle evdi_open(int card) { (void)card; live_device++; return (evdi_handle)(uintptr_t)1; }
void evdi_connect(evdi_handle h, const unsigned char *edid, unsigned int length, uint32_t area) { (void)h; (void)edid; (void)length; (void)area; }
void evdi_disconnect(evdi_handle h) { (void)h; }
void evdi_close(evdi_handle h) { (void)h; live_device--; }
void evdi_register_buffer(evdi_handle h, struct evdi_buffer buffer) { (void)h; assert(buffer.buffer); registered++; }
void evdi_unregister_buffer(evdi_handle h, int id) { (void)h; (void)id; registered--; }
bool evdi_request_update(evdi_handle h, int id) { (void)h; (void)id; return false; }
int evdi_get_event_ready(evdi_handle h) { (void)h; return -1; }
void evdi_handle_events(evdi_handle h, struct evdi_event_context *context) { (void)h; (void)context; }
void evdi_grab_pixels(evdi_handle h, struct evdi_rect *rectangles, int *count) { (void)h; (void)rectangles; *count=0; }

void drmModeFreeResources(drmModeResPtr value) { live_objects--; free(value); }
void drmModeFreeConnector(drmModeConnectorPtr value) { live_objects--; free(value); }
int drmModeRmFB(int fd, uint32_t id) { (void)fd; (void)id; return 0; }
void drmModeFreePlaneResources(drmModePlaneResPtr value) { live_objects--; free(value); }
drmModePlanePtr drmModeGetPlane(int fd, uint32_t id) {
    (void)fd;
    if (scenario == 8) return NULL;
    drmModePlanePtr plane=calloc(1,sizeof(*plane)); live_objects++;
    plane->plane_id=id; plane->possible_crtcs=1; return plane;
}
void drmModeFreePlane(drmModePlanePtr value) { live_objects--; free(value); }
drmModeObjectPropertiesPtr drmModeObjectGetProperties(int fd, uint32_t id, uint32_t type) {
    (void)fd; (void)type;
    if (scenario == 9) return NULL;
    static uint32_t property=1; static uint64_t value;
    value=id == 41 && scenario != 4 ? DRM_PLANE_TYPE_CURSOR : DRM_PLANE_TYPE_PRIMARY;
    drmModeObjectPropertiesPtr properties=calloc(1,sizeof(*properties)); live_objects++;
    properties->count_props=1; properties->props=&property; properties->prop_values=&value;
    return properties;
}
void drmModeFreeObjectProperties(drmModeObjectPropertiesPtr value) { live_objects--; free(value); }
drmModePropertyPtr drmModeGetProperty(int fd, uint32_t id) {
    (void)fd; (void)id;
    drmModePropertyPtr property=calloc(1,sizeof(*property)); live_objects++;
    strcpy(property->name,"type"); return property;
}
void drmModeFreeProperty(drmModePropertyPtr value) { live_objects--; free(value); }

static void check_probe_result(int status, int expected, const char *text) {
    assert(WIFEXITED(status) && WEXITSTATUS(status)==expected);
    if (expected && expected != 78) assert(!strstr(text,"T753 cursor shown"));
    if (expected) assert(!strstr(text,"T753 cursor hidden"));
}

static void probe_case(int which, int expected) {
    scenario=which;
    int output[2]; assert(pipe(output)==0);
    pid_t child=fork(); assert(child>=0);
    if (!child) {
        close(output[0]); assert(dup2(output[1], STDOUT_FILENO)>=0); close(output[1]);
        int result=probe_main();
        assert(!live_objects && !live_buffers && !live_device && !registered);
        exit(result);
    }
    close(output[1]);
    char text[4096]={0}; ssize_t used=0, n;
    while ((n=read(output[0],text+used,sizeof(text)-1-(size_t)used))>0) used+=n;
    close(output[0]);
    int status; assert(waitpid(child,&status,0)==child);
    check_probe_result(status, expected, text);
}
int main(void) {
    probe_case(1,75); /* Null capture storage. */
    probe_case(2,76); probe_case(3,76); probe_case(4,76); /* Missing/empty/noncursor planes. */
    probe_case(5,73); probe_case(6,77); probe_case(7,78); /* Failed native transitions. */
    probe_case(8,76); probe_case(9,76); /* Missing plane/properties. */
    probe_case(0,0); /* Cursor is first, never assumed at index one. */
    return 0;
}
