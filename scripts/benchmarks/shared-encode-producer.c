/* T418: actual conversion and raw transport, without attaching a display. */
#define _GNU_SOURCE
#include "raw_ring.h"
#include "conversion.h"
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <poll.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

static int argument(const char *text, int minimum, int maximum, int *value) {
    char *end;
    errno = 0;
    long number = strtol(text, &end, 10);
    if (errno || end == text || *end || number < minimum || number > maximum) return 0;
    *value = (int)number;
    return 1;
}

typedef struct { int width, height, count, fps, threads, socket, shared; } producer_options;

static int geometry(char **argv, producer_options *options) {
    if (!argument(argv[3], 2, 4096, &options->width) ||
        !argument(argv[4], 2, 4096, &options->height)) return 0;
    return !((options->width | options->height) & 1);
}

static int options_parse(int argc, char **argv, producer_options *options) {
    if (argc != 10) return 0;
    options->shared = strcmp(argv[1], "shared") == 0;
    if (!options->shared && strcmp(argv[1], "fifo")) return 0;
    if (!geometry(argv, options)) return 0;
    return argument(argv[5], 1, INT_MAX - 20, &options->count) &&
        argument(argv[6], 1, 240, &options->fps) &&
        argument(argv[7], 1, MAX_CONV_THREADS, &options->threads) &&
        argument(argv[9], options->shared ? 0 : -1, INT_MAX, &options->socket);
}

static uint64_t now_ns(void) {
    struct timespec now; assert(clock_gettime(CLOCK_MONOTONIC, &now) == 0);
    return (uint64_t)now.tv_sec * 1000000000 + now.tv_nsec;
}
static void wait_until(uint64_t deadline) {
    struct timespec when = {deadline / 1000000000, deadline % 1000000000};
    while (clock_nanosleep(CLOCK_MONOTONIC, TIMER_ABSTIME, &when, NULL) == EINTR) {}
}
static void write_frame(int fd, unsigned char *bytes, size_t size) {
    while (size) {
        ssize_t count = write(fd, bytes, size);
        if (count < 0 && errno == EINTR) continue;
        assert(count > 0); bytes += count; size -= (size_t)count;
    }
}
static void pattern(unsigned char *bgra, int width, int height, int sequence) {
    /* Repeatable moving gradients exercise full conversion and prediction.
     * Source generation is excluded; it substitutes for an already grabbed FB. */
    for (int y = 0; y < height; y++) for (int x = 0; x < width; x++) {
        unsigned char *pixel = bgra + ((size_t)y * width + x) * 4;
        pixel[0] = (x + (uint64_t)sequence * 3) / 4;
        pixel[1] = (y + (uint64_t)sequence * 2) / 4;
        pixel[2] = (x + y + (uint64_t)sequence) / 8;
        pixel[3] = 255;
    }
}
static unsigned char *reserve(raw_ring_t *ring, uint32_t *slot) {
    unsigned char *pixels;
    uint64_t deadline = now_ns() + 2000000000;
    while (!(pixels = raw_ring_acquire(ring, slot))) {
        assert(now_ns() < deadline);
        struct pollfd socket = {.fd=ring->socket, .events=POLLIN};
        poll(&socket, 1, 10);
        assert(raw_ring_service(ring));
    }
    return pixels;
}
int main(int argc, char **argv) {
    producer_options options = {0};
    if (!options_parse(argc, argv, &options)) {
        fprintf(stderr, "Invalid producer arguments: even dimensions 2..4096, positive count, fps 1..240, workers 1..128 required\n");
        return 2;
    }
    int shared = options.shared, width = options.width, height = options.height, count = options.count;
    int fps = options.fps, threads = options.threads, socket = options.socket;
    alarm(count / fps + 20);
    size_t size = (size_t)width * height * 3 / 2;
    unsigned char *bgra = malloc((size_t)width * height * 4), *packed = malloc(size);
    assert(bgra && packed);
    conv_pool_t pool = CONV_POOL_INITIALIZER;
    conv_pool_start(&pool, threads);
    raw_ring_t ring = RAW_RING_INITIALIZER;
    int fifo = -1;
    if (shared) {
        assert(raw_ring_init(&ring, socket));
        assert(raw_ring_resize(&ring, width, height));
    } else {
        fifo = open(argv[2], O_WRONLY); assert(fifo >= 0);
        fcntl(fifo, F_SETPIPE_SZ, 4 * 1024 * 1024);
    }
    fprintf(stderr, "producer: %s %dx%d fps=%d workers=%d fifo_capacity=%d\n",
        argv[1], width, height, fps, pool.count, shared ? 0 : fcntl(fifo, F_GETPIPE_SZ));
    uint64_t start = now_ns();
    for (int sequence = 0; sequence < count; sequence++) {
        pattern(bgra, width, height, sequence);
        wait_until(start + (uint64_t)sequence * 1000000000 / fps);
        uint64_t captured = now_ns();
        uint32_t slot = 0;
        unsigned char *pixels = shared ? reserve(&ring, &slot) : packed;
        conv_job_t job = {bgra, pixels, pixels + width * height, width, height, width * 4,
            width, height, 1, 0, height / 2, NULL, NULL};
        conv_pool_convert(&pool, &job);
        printf("%d %llu %llu\n", sequence, (unsigned long long)captured, (unsigned long long)now_ns());
        fflush(stdout);
        if (shared) assert(raw_ring_publish(&ring, slot, captured / 1000) == 1);
        else write_frame(fifo, pixels, size);
    }
    if (shared) raw_ring_close(&ring);
    else { close(fifo); close(socket); }
    conv_pool_destroy(&pool); free(bgra); free(packed);
    return 0;
}
