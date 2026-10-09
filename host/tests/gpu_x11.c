/* T575 optional native adapter suite. Run in an isolated Xvfb server: never
 * moves the real desktop pointer, changes a monitor, or needs a GPU. */
#include "../gpu/gpu.h"
#include <X11/Xutil.h>
#include <X11/extensions/Xrender.h>
#include <assert.h>
#include <string.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>

/* T770: inspect only Blent's native boundary, on the private X server. */
static int fetched_rectangles;
static unsigned pictures_created;
static unsigned cursor_images;
XFixesCursorImage *__real_XFixesGetCursorImage(Display *);
XFixesCursorImage *__wrap_XFixesGetCursorImage(Display *display) {
    cursor_images++;
    return __real_XFixesGetCursorImage(display);
}
Picture __real_XRenderCreatePicture(Display *, Drawable, const XRenderPictFormat *, unsigned long, const XRenderPictureAttributes *);
Picture __wrap_XRenderCreatePicture(Display *display, Drawable drawable, const XRenderPictFormat *format,
        unsigned long mask, const XRenderPictureAttributes *attributes) {
    pictures_created++;
    return __real_XRenderCreatePicture(display, drawable, format, mask, attributes);
}
static int fragmented_region;
void __real_XDamageSubtract(Display *, Damage, XserverRegion, XserverRegion);
void __wrap_XDamageSubtract(Display *display, Damage damage, XserverRegion repair, XserverRegion parts) {
    __real_XDamageSubtract(display, damage, repair, parts);
    if (!fragmented_region) return;
    XRectangle rectangles[192];
    for (int i = 0; i < 192; i++) rectangles[i] = (XRectangle){20 + i % 16 * 2, 20 + i / 16 * 2, 1, 1};
    XFixesSetRegion(display, parts, rectangles, 192);
}
XRectangle *__real_XFixesFetchRegion(Display *, XserverRegion, int *);
XRectangle *__wrap_XFixesFetchRegion(Display *display, XserverRegion region, int *count) {
    XRectangle *result = __real_XFixesFetchRegion(display, region, count);
    if (*count > fetched_rectangles) fetched_rectangles = *count;
    return result;
}

static void fragmented_damage(gpu_capture *c, GC gc) {
    fetched_rectangles = 0;
    fragmented_region = 1;
    for (int y = 0; y < c->height; y += 2)
        for (int x = 0; x < c->width; x += 2)
            XFillRectangle(c->display, c->root, gc, c->x + x, c->y + y, 1, 1);
    XSync(c->display, False);
    gpu_events_wait(c, 0, 30);
    fragmented_region = 0;
    assert(fetched_rectangles == 1 && "T770: fragmented damage must have bounded transfer and scan");
}

static void outside_pointer(gpu_capture *c) {
    XWarpPointer(c->display, None, c->root, 0, 0, 0, 0, 100, 100);
    XSync(c->display, False);
    gpu_events_wait(c, 0, 30); /* Exit clears the previous visible cursor. */
    XWarpPointer(c->display, None, c->root, 0, 0, 0, 0, 104, 104);
    XSync(c->display, False);
    uint64_t start = gpu_now_ns();
    gpu_events_wait(c, start, 30);
    assert(gpu_now_ns() - start >= 190000000 && "T771: unrelated pointer motion must retain idle cadence");
    XWarpPointer(c->display, None, c->root, 0, 0, 0, 0, 24, 24);
    XSync(c->display, False);
    start = gpu_now_ns();
    gpu_events_wait(c, start - 40000000, 30);
    assert(gpu_now_ns() - start < 150000000); /* Re-entry still wakes capture. */
}

static void codec_error(void) {
    pid_t pid = fork(); assert(pid >= 0);
    if (!pid) { gpu_av(AVERROR(EINVAL), "T575 forced codec rejection"); _exit(0); }
    int status; assert(waitpid(pid, &status, 0) == pid);
    assert(WIFEXITED(status) && WEXITSTATUS(status) == 2);
    gpu_av(0, "T575 successful codec operation");
}

static void fill(gpu_capture *c) {
    XSetForeground(c->display, c->gc, 0);
    XFillRectangle(c->display, c->pixmap, c->gc, 0, 0, c->width, c->height);
}

static void verify(gpu_capture *c, int present) {
    XImage *image = XGetImage(c->display, c->pixmap, 0, 0, c->width, c->height, AllPlanes, ZPixmap);
    assert(image);
    for (int y = 0; y < c->height; y++) {
        for (int x = 0; x < c->width; x++) {
            unsigned long color = present == 2 ? 0xff : 0xff0000;
            unsigned long expected = present && x < 7 && y < 8 ? color : 0;
            assert((XGetPixel(image, x, y) & 0xffffff) == expected);
        }
    }
    XDestroyImage(image);
}

static Cursor cursor(Display *display, Window root, int blue) {
    char bits[8]; memset(bits, 255, sizeof(bits));
    Pixmap source = XCreateBitmapFromData(display, root, bits, 8, 8);
    XColor foreground = {.red=blue ? 0 : 65535, .blue=blue ? 65535 : 0}, background = {0};
    Cursor cursor = XCreatePixmapCursor(display, source, source, &foreground, &background, 3, 2);
    XFreePixmap(display, source);
    return cursor;
}

static void changed_cursor(gpu_capture *c, Cursor original) {
    unsigned before = pictures_created;
    Cursor blue = cursor(c->display, c->root, 1);
    XDefineCursor(c->display, c->root, blue); XSync(c->display, False);
    fill(c); gpu_cursor(c); verify(c, 2);
    assert(pictures_created == before + 1); /* Replace source, retain destination. */
    XDefineCursor(c->display, c->root, original); XSync(c->display, False);
    fill(c); gpu_cursor(c); verify(c, 1);
    assert(pictures_created == before + 2);
    XFreeCursor(c->display, blue);
}

static void t791_offscreen_shape(gpu_capture *c, Cursor original) {
    Cursor blue = cursor(c->display, c->root, 1);
    XWarpPointer(c->display, None, c->root, 0, 0, 0, 0, 100, 100);
    XDefineCursor(c->display, c->root, blue); XSync(c->display, False);
    fill(c); gpu_cursor(c); verify(c, 0);
    XWarpPointer(c->display, None, c->root, 0, 0, 0, 0, 22, 22); XSync(c->display, False);
    fill(c); gpu_cursor(c); verify(c, 2); /* T791: old Picture cannot outlive new metadata. */
    XDefineCursor(c->display, c->root, original); XSync(c->display, False);
    fill(c); gpu_cursor(c); verify(c, 1);
    XFreeCursor(c->display, blue);
}

static void t791_event_flood(gpu_capture *c) {
    XEvent event = {.type=PropertyNotify};
    for (int i = 0; i < 300; i++) XPutBackEvent(c->display, &event);
    unsigned before = cursor_images;
    fill(c); gpu_cursor(c); verify(c, 1);
    assert(cursor_images == before + 1 && "T791: unfinished event drain requires fresh cursor pixels");
    fill(c); gpu_cursor(c); verify(c, 1);
}

/* T579: native event plumbing on a private X server, without GPU imports. */
static void cadence(gpu_capture *c) {
    gpu_options options = {.fps=30};
    setenv("BLENT_GPU_CADENCE", "damage", 1);
    gpu_events_open(c, &options);
    XSync(c->display, False);
    gpu_events_wait(c, 0, 30); /* Drain initial full damage and pointer state. */
    uint64_t start = gpu_now_ns();
    gpu_events_wait(c, start, 30);
    assert(gpu_now_ns() - start >= 190000000); /* Idle refresh, no busy loop. */
    GC root_gc = XCreateGC(c->display, c->root, 0, NULL);
    XSetForeground(c->display, root_gc, 0x778899);
    XFillRectangle(c->display, c->root, root_gc, c->x, c->y, 4, 4);
    XSync(c->display, False);
    start = gpu_now_ns();
    gpu_events_wait(c, start - 40000000, 30);
    assert(gpu_now_ns() - start < 150000000); /* Damage bypasses idle deadline. */
    XWarpPointer(c->display, None, c->root, 0, 0, 0, 0, 24, 24);
    XSync(c->display, False);
    start = gpu_now_ns();
    gpu_events_wait(c, start - 40000000, 30);
    assert(gpu_now_ns() - start < 150000000); /* Cursor-only motion. */
    XFillRectangle(c->display, c->root, root_gc, 110, 110, 4, 4);
    XSync(c->display, False);
    start = gpu_now_ns();
    gpu_events_wait(c, start, 30);
    assert(gpu_now_ns() - start >= 190000000); /* Foreign output damage ignored. */
    fragmented_damage(c, root_gc);
    outside_pointer(c);
    pid_t child = fork(); assert(child >= 0);
    if (!child) { c->leased = 1; gpu_events_wait(c, 0, 30); _exit(0); }
    int status; assert(waitpid(child, &status, 0) == child);
    assert(WIFEXITED(status) && WEXITSTATUS(status) == 2);
    XFreeGC(c->display, root_gc);
    gpu_events_close(c);
    c->damage = 0; /* This test explicitly retired events before capture teardown. */
    unsetenv("BLENT_GPU_CADENCE");
}

int main(void) {
    codec_error();
    gpu_capture c = {.x=20, .y=20, .width=32, .height=24, .dma_fd=-1, .render_fd=-1};
    c.display = XOpenDisplay(NULL); assert(c.display);
    c.root = DefaultRootWindow(c.display);
    c.pixmap = XCreatePixmap(c.display, c.root, c.width, c.height, 24);
    c.gc = XCreateGC(c.display, c.pixmap, 0, NULL);
    c.fence = XSyncCreateFence(c.display, c.pixmap, False);
    Cursor shape = cursor(c.display, c.root, 0);
    XDefineCursor(c.display, c.root, shape);
    XWarpPointer(c.display, None, c.root, 0, 0, 0, 0, 22, 22); XSync(c.display, False);
    fill(&c); gpu_cursor(&c); verify(&c, 1);
    unsigned created = pictures_created;
    unsigned images = cursor_images;
    for (int i = 0; i < 32; i++) { fill(&c); gpu_cursor(&c); verify(&c, 1); }
    assert(pictures_created == created && "T772: unchanged shape must reuse cursor Pictures");
    assert(cursor_images == images && "T791: unchanged shape must not fetch cursor pixels again");
    changed_cursor(&c, shape);
    t791_offscreen_shape(&c, shape);
    t791_event_flood(&c);
    XWarpPointer(c.display, None, c.root, 0, 0, 0, 0, 100, 100); XSync(c.display, False);
    fill(&c); gpu_cursor(&c); verify(&c, 0);
    cadence(&c);
    gpu_cursor_close(&c);
    gpu_cursor_close(&c); /* Empty cache teardown remains safe. */
    /* Refill and exercise the production capture owner's cache teardown too. */
    fill(&c); gpu_cursor(&c);
    XFreeCursor(c.display, shape);
    gpu_capture_close(&c);
    return 0;
}
