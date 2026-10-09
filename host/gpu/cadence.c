#include "options.h"

int gpu_cursor_intersects(int x, int y, int width, int height,
        int pointer_x, int pointer_y, const gpu_cursor_shape *shape) {
    if (width <= 0 || height <= 0 || shape->width <= 0 || shape->height <= 0) return 0;
    int64_t left = (int64_t)pointer_x - shape->xhot;
    int64_t top = (int64_t)pointer_y - shape->yhot;
    return left < (int64_t)x + width && top < (int64_t)y + height &&
        left + shape->width > x && top + shape->height > y;
}

/* Shared timing/rectangle policy has no X11 or GPU dependency. */
uint64_t gpu_capture_deadline(uint64_t previous, unsigned fps, int dirty) {
    uint64_t period = 1000000000 / fps;
    if (!dirty && period < 200000000) period = 200000000;
    return previous + period;
}

int gpu_intersects(int x, int y, int width, int height, int rx, int ry, int rw, int rh) {
    return (int64_t)rx < (int64_t)x + width && (int64_t)ry < (int64_t)y + height &&
        (int64_t)rx + rw > x && (int64_t)ry + rh > y;
}

int gpu_refresh_due(int64_t previous, int64_t now) {
    return previous == 0 || now - previous >= 900000;
}
