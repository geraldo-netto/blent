#ifndef BLENT_CURSOR_OVERLAY_H
#define BLENT_CURSOR_OVERLAY_H
#include <stddef.h>
#include <stdint.h>
#include "pixel_format.h"

/* Platform-independent, event-thread-owned premultiplied cursor overlay.
 * Images are borrowed only during set(); internal compact pixels are owned.
 * Restore before changing image/position or accepting desktop damage. Forget
 * the saved rectangle before replacing a surface. Surface storage is borrowed
 * until restore; callers validate positive geometry and stride >= width*4.
 * Position is the cursor image's top-left in surface coordinates, not hotspot.
 * Inputs are bounded to one million pixels (the GPU cursor adapter's limit).
 */
typedef struct { int x1, y1, x2, y2; } cursor_rect_t;
typedef struct {
    const unsigned char *pixels;
    size_t length;
    uint32_t width, height, stride;
    int alpha, swap_rb;
} cursor_image_t;
typedef struct {
    unsigned char *storage;
    unsigned width, height;
    int x, y;
    cursor_rect_t drawn;
    pixel_order_t order; /* borrowed surface byte order; image storage stays BGRA */
} cursor_overlay_t;
int cursor_overlay_set(cursor_overlay_t *cursor, const cursor_image_t *image);
void cursor_overlay_destroy(cursor_overlay_t *cursor);
cursor_rect_t cursor_overlay_restore(cursor_overlay_t *cursor, unsigned char *surface, int stride);
cursor_rect_t cursor_overlay_draw(cursor_overlay_t *cursor, unsigned char *surface, int width, int height, int stride);
#endif
