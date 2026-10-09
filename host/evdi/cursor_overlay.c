#include "cursor_overlay.h"
#include <stdlib.h>
#include <string.h>

static int cursor_image_valid(const cursor_image_t *image) {
    if (!image->pixels || !image->width || !image->height) return 0;
    uint64_t pixels = (uint64_t)image->width * image->height;
    uint64_t row = (uint64_t)image->width * 4;
    if (pixels > 1024 * 1024 || row > image->stride) return 0;
    return (uint64_t)image->stride * (image->height - 1) + row <= image->length;
}

void cursor_overlay_destroy(cursor_overlay_t *cursor) {
    free(cursor->storage);
    cursor->storage = NULL;
    cursor->width = cursor->height = 0;
    cursor->drawn = (cursor_rect_t){0};
}

static void cursor_copy_image(unsigned char *destination, const cursor_image_t *image) {
    for (unsigned y = 0; y < image->height; y++) {
        const unsigned char *source = image->pixels + (size_t)y * image->stride;
        for (unsigned x = 0; x < image->width; x++) {
            destination[0] = source[image->swap_rb ? 2 : 0];
            destination[1] = source[1];
            destination[2] = source[image->swap_rb ? 0 : 2];
            destination[3] = image->alpha ? source[3] : 255;
            source += 4; destination += 4;
        }
    }
}

int cursor_overlay_set(cursor_overlay_t *cursor, const cursor_image_t *image) {
    if (!image) { cursor_overlay_destroy(cursor); return 1; }
    if (!cursor_image_valid(image)) return 0;
    unsigned char *storage = malloc((size_t)image->width * image->height * 8);
    if (!storage) return 0;
    cursor_copy_image(storage, image);
    cursor_overlay_destroy(cursor);
    cursor->storage = storage;
    cursor->width = image->width; cursor->height = image->height;
    return 1;
}

cursor_rect_t cursor_overlay_restore(cursor_overlay_t *cursor, unsigned char *surface, int stride) {
    cursor_rect_t rect = cursor->drawn;
    if (!surface || !cursor->storage) return (cursor_rect_t){0};
    const unsigned char *saved = cursor->storage + (size_t)cursor->width * cursor->height * 4;
    for (int y = rect.y1; y < rect.y2; y++) {
        size_t offset = ((size_t)(y - rect.y1) * cursor->width) * 4;
        memcpy(surface + (size_t)y * stride + (size_t)rect.x1 * 4,
               saved + offset, (size_t)(rect.x2 - rect.x1) * 4);
    }
    cursor->drawn = (cursor_rect_t){0};
    return rect;
}

static cursor_rect_t cursor_visible(const cursor_overlay_t *cursor, int width, int height) {
    int64_t x2 = (int64_t)cursor->x + cursor->width;
    int64_t y2 = (int64_t)cursor->y + cursor->height;
    cursor_rect_t rect = {cursor->x < 0 ? 0 : cursor->x, cursor->y < 0 ? 0 : cursor->y,
        x2 > width ? width : (int)x2, y2 > height ? height : (int)y2};
    if (rect.x1 >= rect.x2 || rect.y1 >= rect.y2) return (cursor_rect_t){0};
    return rect;
}

static void cursor_blend(unsigned char *destination, const unsigned char *source, pixel_order_t order) {
    unsigned alpha = source[3];
    if (!alpha) return;
    for (int channel = 0; channel < 3; channel++) {
        int source_channel = order == PIXEL_RGBA ? 2 - channel : channel;
        unsigned value = source[source_channel] + (destination[channel] * (255 - alpha) + 127) / 255;
        destination[channel] = value > 255 ? 255 : value;
    }
    destination[3] = 255;
}

static void cursor_draw_row(cursor_overlay_t *cursor, unsigned char *surface, int stride, int y) {
    cursor_rect_t rect = cursor->drawn;
    unsigned char *row = surface + (size_t)y * stride + (size_t)rect.x1 * 4;
    unsigned char *saved = cursor->storage + (size_t)cursor->width * cursor->height * 4;
    memcpy(saved + (size_t)(y - rect.y1) * cursor->width * 4, row, (size_t)(rect.x2 - rect.x1) * 4);
    const unsigned char *source = cursor->storage +
        ((size_t)((int64_t)y - cursor->y) * cursor->width + (size_t)((int64_t)rect.x1 - cursor->x)) * 4;
    for (int x = rect.x1; x < rect.x2; x++, row += 4, source += 4) cursor_blend(row, source, cursor->order);
}

cursor_rect_t cursor_overlay_draw(cursor_overlay_t *cursor, unsigned char *surface, int width, int height, int stride) {
    if (!cursor->storage || !surface || !pixel_order_valid(cursor->order)) return (cursor_rect_t){0};
    cursor->drawn = cursor_visible(cursor, width, height);
    for (int y = cursor->drawn.y1; y < cursor->drawn.y2; y++) cursor_draw_row(cursor, surface, stride, y);
    return cursor->drawn;
}
