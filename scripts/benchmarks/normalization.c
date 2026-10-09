/* T779: isolated normalization replay, no device or display access.
 * Compile -O3 -ffunction-sections -fdata-sections -Wl,--gc-sections -pthread
 * with host/evdi/conversion.c; override BLENT_CAPTURE_SOURCE for paired trees. */
#ifndef BLENT_CAPTURE_SOURCE
#define BLENT_CAPTURE_SOURCE "../../host/evdi/capture.c"
#endif
#include BLENT_CAPTURE_SOURCE
#include "checked_arguments.h"
#include <assert.h>
#include <inttypes.h>

enum { WIDTH = 1920, HEIGHT = 1080, STRIDE = WIDTH * 4 + 64, SAMPLES = 32 };
static uint64_t stamp(clockid_t clock) {
    struct timespec t; assert(!clock_gettime(clock, &t));
    return (uint64_t)t.tv_sec * 1000000000 + t.tv_nsec;
}
static int compare_stamp(const void *a, const void *b) {
    uint64_t x = *(const uint64_t *)a, y = *(const uint64_t *)b;
    return (x > y) - (x < y);
}
static int rectangles(struct evdi_rect *rects, int kind) {
    int count = kind >= 2 ? 63 : 1;
    for (int i = 0; i < count; i++) {
        rects[i] = (struct evdi_rect){0, 0, WIDTH, HEIGHT};
        if (kind == 0) rects[i] = (struct evdi_rect){50, 50, 114, 114};
        if (kind == 3) rects[i] = (struct evdi_rect){i * 20, 50, i * 20 + 2, 114};
    }
    return count;
}
static void check(capture_context_t *capture) {
    for (int y = 0; y < HEIGHT; y++) {
        for (int x = 0; x < WIDTH; x++) {
            size_t o = (size_t)y * STRIDE + x * 4;
            assert(capture->framebuffer[o] == capture->native_framebuffer[o + 2]);
            assert(capture->framebuffer[o + 1] == capture->native_framebuffer[o + 1]);
            assert(capture->framebuffer[o + 2] == capture->native_framebuffer[o]);
            assert(capture->framebuffer[o + 3] == capture->native_framebuffer[o + 3]);
        }
        for (int x = WIDTH * 4; x < STRIDE; x++) assert(capture->framebuffer[y * STRIDE + x] == 0xA5);
    }
}
static void measure(capture_context_t *capture, struct evdi_rect *rects, int count, int kind) {
    uint64_t samples[SAMPLES];
    for (int i = 0; i < 8; i++) normalize_framebuffer(capture, rects, count);
    uint64_t cpu = stamp(CLOCK_PROCESS_CPUTIME_ID);
    for (int i = 0; i < SAMPLES; i++) {
        uint64_t start = stamp(CLOCK_MONOTONIC);
        normalize_framebuffer(capture, rects, count);
        samples[i] = stamp(CLOCK_MONOTONIC) - start;
    }
    cpu = stamp(CLOCK_PROCESS_CPUTIME_ID) - cpu;
    qsort(samples, SAMPLES, sizeof(*samples), compare_stamp);
    check(capture);
    printf("{\"kind\":%d,\"wall_p50_ns\":%"PRIu64",\"cpu_ns_per_frame\":%"PRIu64",\"samples\":%d}\n",
        kind, samples[SAMPLES / 2], cpu / SAMPLES, SAMPLES);
}
int main(int argc, char **argv) {
    assert(argc == 2);
    int kind = benchmark_number(argv[1], 0, 3);
    capture_context_t capture = {.mode_w=WIDTH, .mode_h=HEIGHT, .mode_stride=STRIDE, .swap_rb=1, .normalize_rgba=1};
    capture.framebuffer = malloc(STRIDE * HEIGHT); capture.native_framebuffer = malloc(STRIDE * HEIGHT);
    assert(capture.framebuffer && capture.native_framebuffer);
    for (size_t i = 0; i < STRIDE * HEIGHT; i++) capture.native_framebuffer[i] = (unsigned char)(i * 17 + i / 997);
    memset(capture.framebuffer, 0xA5, STRIDE * HEIGHT);
    normalize_framebuffer(&capture, NULL, 64);
    struct evdi_rect rects[63]; int count = rectangles(rects, kind);
    measure(&capture, rects, count, kind);
    free(capture.framebuffer); free(capture.native_framebuffer); return 0;
}
