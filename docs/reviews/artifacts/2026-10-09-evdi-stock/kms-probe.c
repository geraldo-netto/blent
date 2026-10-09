/* Guest-only stock API probe. Historical logs predate T765 error handling.
 * No production mitigation or compositor claim. */
#include <xf86drm.h>
#include <xf86drmMode.h>
#include <drm_fourcc.h>
#include "evdi_lib.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <poll.h>
#include <sys/mman.h>
#include "probe_device.h"

static int ready, modes;
static void mode_cb(struct evdi_mode m, void *p) {
    (void)p; modes++; printf("mode %dx%d %.4s\n", m.width, m.height, (char *)&m.pixel_format);
}
static void ready_cb(int b, void *p) { (void)p; (void)b; ready++; }
static struct evdi_event_context events = {.mode_changed_handler=mode_cb, .update_ready_handler=ready_cb};
static void drain(evdi_handle h, int ms) {
    struct pollfd p = {evdi_get_event_ready(h), POLLIN, 0};
    while (poll(&p, 1, ms) > 0) { evdi_handle_events(h, &events); ms=0; }
}
static void grab(evdi_handle h, const char *label, uint32_t *pixels) {
    struct evdi_rect rects[64]; int n=64;
    evdi_grab_pixels(h, rects, &n);
    printf("%s rects=%d modes=%d ready=%d pixel=%08x\n", label, n, modes, ready, pixels[10*640+10]);
}

typedef struct {
    int fd, registered;
    evdi_handle handle;
    drmModeResPtr resources;
    drmModeConnectorPtr connector;
    struct drm_mode_create_dumb scanout, cursor;
    uint32_t fb, cursor_fb, plane;
    uint32_t *pixels;
} probe_capture;

static int create_buffer(int fd, int width, int height, uint32_t color, struct drm_mode_create_dumb *buffer) {
    *buffer=(struct drm_mode_create_dumb){.width=width, .height=height, .bpp=32};
    if (drmIoctl(fd, DRM_IOCTL_MODE_CREATE_DUMB, buffer)) return 70;
    struct drm_mode_map_dumb map={.handle=buffer->handle};
    if (drmIoctl(fd, DRM_IOCTL_MODE_MAP_DUMB, &map)) return 71;
    uint32_t *pixels=mmap(NULL, buffer->size, PROT_READ|PROT_WRITE, MAP_SHARED, fd, map.offset);
    if (pixels == MAP_FAILED) return 72;
    for (uint64_t i=0; i<buffer->size/4; i++) pixels[i]=color;
    munmap(pixels, buffer->size);
    return 0;
}
static int add_framebuffer(probe_capture *c, const struct drm_mode_create_dumb *buffer, uint32_t format, uint32_t *fb) {
    uint32_t handles[4]={buffer->handle}, pitches[4]={buffer->pitch}, offsets[4]={0};
    return drmModeAddFB2(c->fd, buffer->width, buffer->height, format, handles, pitches, offsets, fb, 0) ? 73 : 0;
}
static void destroy_buffer(int fd, struct drm_mode_create_dumb buffer) {
    if (!buffer.handle) return;
    struct drm_mode_destroy_dumb destroy={.handle=buffer.handle};
    drmIoctl(fd, DRM_IOCTL_MODE_DESTROY_DUMB, &destroy);
}
static void release_drm(probe_capture *c) {
    if (c->plane && c->resources)
        drmModeSetPlane(c->fd, c->plane, c->resources->crtcs[0], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
    if (c->fb && c->resources) drmModeSetCrtc(c->fd, c->resources->crtcs[0], 0, 0, 0, NULL, 0, NULL);
    if (c->cursor_fb) drmModeRmFB(c->fd, c->cursor_fb);
    if (c->fb) drmModeRmFB(c->fd, c->fb);
    destroy_buffer(c->fd, c->cursor); destroy_buffer(c->fd, c->scanout);
    if (c->connector) drmModeFreeConnector(c->connector);
    if (c->resources) drmModeFreeResources(c->resources);
    close(c->fd);
}
static void release_capture(probe_capture *c) {
    if (c->handle) {
        if (c->registered) evdi_unregister_buffer(c->handle, 0);
        evdi_disconnect(c->handle); evdi_close(c->handle);
    }
    free(c->pixels);
    release_drm(c);
}
static int read_edid(unsigned char edid[128]) {
    FILE *file=fopen("/edid.bin", "rb");
    if (!file) return 0;
    int valid=fread(edid, 1, 128, file)==128;
    fclose(file);
    return valid;
}
static int connect_capture(probe_capture *c, int card, const unsigned char edid[128]) {
    c->handle=evdi_open(card);
    if (!c->handle) return 67;
    evdi_connect(c->handle, edid, 128, 0);
    c->resources=drmModeGetResources(c->fd);
    if (!c->resources || c->resources->count_connectors<1 || c->resources->count_crtcs<1) return 68;
    c->connector=drmModeGetConnector(c->fd, c->resources->connectors[0]);
    if (!c->connector || c->connector->count_modes<1) return 69;
    printf("connector modes=%d first=%dx%d\n", c->connector->count_modes, c->connector->modes[0].hdisplay, c->connector->modes[0].vdisplay);
    return 0;
}
static int start_scanout(probe_capture *c) {
    int error=create_buffer(c->fd, 640, 480, 0xff000080, &c->scanout);
    if (error) return error;
    error=add_framebuffer(c, &c->scanout, DRM_FORMAT_XRGB8888, &c->fb);
    if (error) return error;
    if (drmModeSetCrtc(c->fd, c->resources->crtcs[0], c->fb, 0, 0, &c->connector->connector_id, 1, &c->connector->modes[0])) return 74;
    c->pixels=calloc(640*480, 4);
    if (!c->pixels) return 75;
    struct evdi_buffer buffer={.id=0, .buffer=c->pixels, .width=640, .height=480, .stride=640*4};
    evdi_register_buffer(c->handle, buffer); c->registered=1;
    return 0;
}
static int cursor_type(int fd, uint32_t id) {
    drmModeObjectPropertiesPtr properties=drmModeObjectGetProperties(fd, id, DRM_MODE_OBJECT_PLANE);
    if (!properties) return 0;
    int cursor=0;
    for (uint32_t i=0; i<properties->count_props; i++) {
        drmModePropertyPtr property=drmModeGetProperty(fd, properties->props[i]);
        if (property) {
            if (!strcmp(property->name, "type") && properties->prop_values[i]==DRM_PLANE_TYPE_CURSOR) cursor=1;
            drmModeFreeProperty(property);
        }
    }
    drmModeFreeObjectProperties(properties);
    return cursor;
}
static uint32_t cursor_plane(int fd) {
    if (drmSetClientCap(fd, DRM_CLIENT_CAP_UNIVERSAL_PLANES, 1)) return 0;
    drmModePlaneResPtr planes=drmModeGetPlaneResources(fd);
    if (!planes) return 0;
    uint32_t selected=0;
    for (uint32_t i=0; i<planes->count_planes; i++) {
        drmModePlanePtr plane=drmModeGetPlane(fd, planes->planes[i]);
        if (!plane) continue;
        if ((plane->possible_crtcs & 1) && cursor_type(fd, plane->plane_id)) selected=plane->plane_id;
        drmModeFreePlane(plane);
        if (selected) break;
    }
    drmModeFreePlaneResources(planes);
    return selected;
}
static int cursor_trial(probe_capture *c) {
    c->plane=cursor_plane(c->fd);
    if (!c->plane) return 76;
    int error=create_buffer(c->fd, 64, 64, 0xffffffff, &c->cursor);
    if (error) return error;
    error=add_framebuffer(c, &c->cursor, DRM_FORMAT_ARGB8888, &c->cursor_fb);
    if (error) return error;
    if (drmModeSetPlane(c->fd, c->plane, c->resources->crtcs[0], c->cursor_fb, 0, 0, 0, 64, 64, 0, 0, 64<<16, 64<<16)) return 77;
    drain(c->handle, 300); grab(c->handle, "T753 cursor shown", c->pixels);
    if (drmModeSetPlane(c->fd, c->plane, c->resources->crtcs[0], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)) return 78;
    printf("T753 hide request=%d\n", evdi_request_update(c->handle, 0));
    drain(c->handle, 300); grab(c->handle, "T753 cursor hidden", c->pixels);
    return 0;
}
static int run_probe(probe_capture *c, int card) {
    unsigned char edid[128];
    if (!read_edid(edid)) return 66;
    int error=connect_capture(c, card, edid);
    if (error) return error;
    error=start_scanout(c);
    if (error) return error;
    drain(c->handle, 300);
    printf("initial immediate=%d\n", evdi_request_update(c->handle, 0));
    drain(c->handle, 100); grab(c->handle, "initial", c->pixels);
    printf("T748 idle request=%d\n", evdi_request_update(c->handle, 0));
    drain(c->handle, 300); grab(c->handle, "T748 watchdog", c->pixels);
    printf("T748 retry=%d\n", evdi_request_update(c->handle, 0));
    error=cursor_trial(c);
    if (error) return error;
    evdi_disconnect(c->handle); evdi_connect(c->handle, edid, 128, 0);
    drain(c->handle, 300); printf("T749 reconnect request=%d\n", evdi_request_update(c->handle, 0));
    drain(c->handle, 300); grab(c->handle, "T749 reconnect no modeset", c->pixels);
    return 0;
}
int main(void) {
    setbuf(stdout, NULL); probe_require_guest();
    int card; drmVersionPtr version=NULL;
    probe_capture capture={.fd=probe_open_evdi(&card, &version)};
    printf("KMS stock %d.%d.%d\n", version->version_major, version->version_minor, version->version_patchlevel);
    drmFreeVersion(version);
    int status=run_probe(&capture, card);
    release_capture(&capture);
    if (status) fprintf(stderr, "KMS probe failed at stage %d; no success claim for that stage\n", status);
    return status;
}
