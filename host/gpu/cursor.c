#include "gpu.h"
#include <X11/Xutil.h>
#include <X11/extensions/Xfixes.h>
#include <X11/extensions/Xrender.h>
#include <stdlib.h>

static int intersects(const gpu_capture *c, const XFixesCursorImage *cursor) {
    int x = cursor->x - cursor->xhot - c->x;
    int y = cursor->y - cursor->yhot - c->y;
    return x < c->width && y < c->height && x + cursor->width > 0 && y + cursor->height > 0;
}

static XImage *cursor_image(gpu_capture *c, const XFixesCursorImage *cursor) {
    unsigned count = (unsigned)cursor->width * cursor->height;
    gpu_require(count <= 1024 * 1024, "bounded cursor image");
    char *pixels = calloc(count, 4); gpu_require(pixels != NULL, "cursor allocation");
    XImage *image = XCreateImage(c->display, DefaultVisual(c->display, DefaultScreen(c->display)),
        32, ZPixmap, 0, pixels, cursor->width, cursor->height, 32, 0);
    gpu_require(image != NULL, "cursor XImage");
    for (unsigned y = 0; y < cursor->height; y++)
        for (unsigned x = 0; x < cursor->width; x++)
            XPutPixel(image, x, y, cursor->pixels[y * cursor->width + x]);
    return image;
}

static Picture cursor_picture(gpu_capture *c, const XFixesCursorImage *cursor) {
    XImage *image = cursor_image(c, cursor);
    Pixmap pixmap = XCreatePixmap(c->display, c->root, cursor->width, cursor->height, 32);
    GC gc = XCreateGC(c->display, pixmap, 0, NULL);
    XPutImage(c->display, pixmap, gc, image, 0, 0, 0, 0, cursor->width, cursor->height);
    XRenderPictFormat *argb = XRenderFindStandardFormat(c->display, PictStandardARGB32);
    gpu_require(argb != NULL, "cursor source format");
    Picture source = XRenderCreatePicture(c->display, pixmap, argb, 0, NULL);
    /* The Picture retains the server pixmap; temporary upload resources can go. */
    XFreeGC(c->display, gc); XFreePixmap(c->display, pixmap); XDestroyImage(image);
    return source;
}

static void cache_cursor(gpu_capture *c, const XFixesCursorImage *cursor) {
    if (!c->cursor_destination) {
        XRenderPictFormat *rgb = XRenderFindVisualFormat(c->display, DefaultVisual(c->display, DefaultScreen(c->display)));
        gpu_require(rgb != NULL, "cursor destination format");
        c->cursor_destination = XRenderCreatePicture(c->display, c->pixmap, rgb, 0, NULL);
    }
    if (c->cursor_source && c->cursor_serial == cursor->cursor_serial) return;
    c->cursor_source = cursor_picture(c, cursor);
    c->cursor_serial = cursor->cursor_serial;
}

static void composite(gpu_capture *c, XFixesCursorImage *cursor) {
    cache_cursor(c, cursor);
    XRenderComposite(c->display, PictOpOver, c->cursor_source, None, c->cursor_destination, 0, 0, 0, 0,
        cursor->x - cursor->xhot - c->x, cursor->y - cursor->yhot - c->y, cursor->width, cursor->height);
}

void gpu_cursor_close(gpu_capture *c) {
    if (c->cursor_source) XRenderFreePicture(c->display, c->cursor_source);
    if (c->cursor_destination) XRenderFreePicture(c->display, c->cursor_destination);
    c->cursor_source = c->cursor_destination = 0;
    if (c->cursor_subscribed) XFixesSelectCursorInput(c->display, c->root, 0);
    c->cursor_subscribed = 0;
    c->cursor_invalid = 1;
}

static void watch_cursor(gpu_capture *c) {
    if (c->cursor_subscribed) return;
    int error;
    gpu_require(XFixesQueryExtension(c->display, &c->cursor_event, &error), "XFixes cursor extension");
    XFixesSelectCursorInput(c->display, c->root, XFixesDisplayCursorNotifyMask);
    c->cursor_subscribed = c->cursor_invalid = 1;
}

static int cached_cursor(gpu_capture *c, XFixesCursorImage *cursor) {
    Window root, child;
    int x, y, local_x, local_y; unsigned mask;
    /* The synchronous reply orders earlier shape notifications before this
     * position sample. Drain those notifications before trusting cached pixels.
     * Unprocessed events force a full image query instead of a stale draw. */
    gpu_require(XQueryPointer(c->display, c->root, &root, &child, &x, &y,
        &local_x, &local_y, &mask), "cursor position");
    if (!gpu_events_drain(c) || c->cursor_invalid || !c->cursor_source) return 0;
    *cursor = (XFixesCursorImage){.x=x, .y=y, .width=c->cursor_shape.width,
        .height=c->cursor_shape.height, .xhot=c->cursor_shape.xhot,
        .yhot=c->cursor_shape.yhot, .cursor_serial=c->cursor_serial};
    return 1;
}

static void refresh_cursor(gpu_capture *c) {
    XFixesCursorImage *cursor = XFixesGetCursorImage(c->display);
    gpu_require(cursor != NULL, "XFixes cursor image");
    /* A shape can change while outside this output. Retire its old Picture
     * before publishing new dimensions/hotspot, even without an upload. */
    if (c->cursor_source && c->cursor_serial != cursor->cursor_serial) {
        XRenderFreePicture(c->display, c->cursor_source);
        c->cursor_source = 0;
    }
    c->cursor_shape = (gpu_cursor_shape){cursor->width, cursor->height, cursor->xhot, cursor->yhot};
    if (intersects(c, cursor)) composite(c, cursor);
    c->cursor_invalid = 0;
    XFree(cursor);
}

void gpu_cursor(gpu_capture *c) {
    watch_cursor(c);
    /* Capture's earlier round trips often queued shape notifications already.
     * Refresh known changes directly, avoiding a redundant position request. */
    if (!gpu_events_drain(c) || c->cursor_invalid || !c->cursor_source) {
        refresh_cursor(c);
        return;
    }
    XFixesCursorImage cursor;
    if (!cached_cursor(c, &cursor)) { refresh_cursor(c); return; }
    if (intersects(c, &cursor)) composite(c, &cursor);
}
