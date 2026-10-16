#define _GNU_SOURCE
#include "capture.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include <poll.h>
#include <errno.h>
#include <limits.h>
#include <sys/mman.h>

static long long capture_now_us(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (long long)ts.tv_sec * 1000000 + ts.tv_nsec / 1000;
}
static long long capture_now_ms(void) { return capture_now_us() / 1000; }

static int cmp_int(const void *a, const void *b) {
    int x = *(const int *)a, y = *(const int *)b;
    return (x > y) - (x < y);
}

static void on_dpms(int dpms_mode, void *user_data) {
    capture_context_t *capture = user_data;
    capture->dpms_on = (dpms_mode == 0) ? 1 : 0;
    fprintf(stderr, "[evdi-helper] DPMS: %d (%s)\n", dpms_mode, capture->dpms_on ? "ON" : "OFF");
}

static pixel_order_t capture_pixel_order(const capture_context_t *capture) {
    return capture->swap_rb && !capture->normalize_rgba ? PIXEL_RGBA : PIXEL_BGRA;
}

static void capture_to_nv12(capture_context_t *capture, const unsigned char *src, unsigned char *dst,
                         const unsigned char *dirty, const pixel_row_t *spans) {
    frame_target_t target = frame_exchange_target(capture->frames);
    conv_job_t frame = {src, dst, dst + (size_t)target.width * target.height,
        capture->mode_w, capture->mode_h, capture->mode_stride, target.width, target.height,
        capture->scale, 0, target.height / 2, dirty,
        spans, capture_pixel_order(capture)};
    conv_pool_convert(capture->conversion, &frame);
}

static void mark_all_dirty(capture_context_t *capture) {
    if (capture->raw_ring) raw_ring_mark_all(capture->raw_ring);
    else frame_exchange_invalidate(capture->frames);
}

typedef struct {
    capture_context_t *capture;
    const struct evdi_rect *rects;
    int count;
} capture_damage_t;
static void apply_damage(frame_damage_t *sink, void *context) {
    capture_damage_t *damage = context;
    capture_context_t *capture = damage->capture;
    for (int i = 0; i < damage->count; i++) {
        const struct evdi_rect *rect = &damage->rects[i];
        if (capture->raw_ring)
            raw_ring_damage(capture->raw_ring, rect->x1, rect->y1,
                            rect->x2, rect->y2, capture->scale);
        else frame_damage_rect(sink, rect->x1, rect->y1, rect->x2, rect->y2, capture->scale);
    }
}
static void mark_damage(capture_context_t *capture, const struct evdi_rect *rects, int n) {
    capture_damage_t damage = {capture, rects, n};
    if (capture->raw_ring) apply_damage(NULL, &damage);
    else frame_exchange_update(capture->frames, apply_damage, &damage);
}

static void cursor_damage(capture_context_t *capture, cursor_rect_t rect) {
    if (rect.x1 >= rect.x2 || rect.y1 >= rect.y2) return;
    struct evdi_rect damage = {rect.x1, rect.y1, rect.x2, rect.y2};
    mark_damage(capture, &damage, 1);
}

static void restore_cursor(capture_context_t *capture) {
    cursor_damage(capture, cursor_overlay_restore(&capture->cursor, capture->framebuffer, capture->mode_stride));
}

static void draw_cursor(capture_context_t *capture) {
    cursor_damage(capture, cursor_overlay_draw(&capture->cursor, capture->framebuffer,
        capture->mode_w, capture->mode_h, capture->mode_stride));
}

static int capture_active(const capture_context_t *capture)
{
    if (!capture->control)
        return 1;
    return capture->control->current.active;
}

static void publish_frame(capture_context_t *capture)
{
    if (!capture_active(capture))
        return;
    if (!frame_exchange_is_ready(capture->frames) || !capture->framebuffer)
        return;
    if (capture->raw_ring)
    {
        capture->raw_pending = 1;
        return;
    }

    unsigned generation;
    if (!frame_exchange_begin(capture->frames, &generation))
        return;
    /* Reconnecting FIFO readers need a full refresh, otherwise convert only
     * the chroma-aligned regions this particular buffer is missing. */
    frame_target_t target = frame_exchange_target(capture->frames);
    capture_to_nv12(capture, capture->framebuffer, target.data, target.dirty, target.spans);
    frame_exchange_publish(capture->frames, capture->grab_us, generation);
}

static void reject_mode(capture_context_t *capture) {
    capture->have_mode = 0;
    frame_exchange_set_ready(capture->frames, 0);
    capture->capture_failed = 1;
    (*capture->running) = 0;
}

static int validate_frame_format(capture_context_t *capture, struct evdi_mode mode) {
    /* Validated byte order is passed to conversion/cursor composition; only
       canonical fallback normalizes RGBA. Packed 10-bit layouts are unsupported. */
    if (mode.bits_per_pixel != 32 ||
            (mode.pixel_format != 0x34325258 && mode.pixel_format != 0x34325241 &&
             mode.pixel_format != 0x34324258 && mode.pixel_format != 0x34324241)) {
        fprintf(stderr, "[evdi-helper] Unsupported framebuffer format; need XRGB/ARGB/XBGR/ABGR8888\n");
        reject_mode(capture);
        return 0;
    }
    return 1;
}

static int validate_mode_dimensions(capture_context_t *capture, struct evdi_mode mode) {
    /* Each output chroma sample reads a complete 2*scale source block.
       Enlarging a tiny output to 2x2 would read outside the source image. */
    if (capture->scale < 1 || capture->scale > 4 || mode.width < 2 * capture->scale || mode.height < 2 * capture->scale ||
            mode.width > (INT_MAX - 63) / 4 ||
            (((long long)mode.width * 4 + 63) & ~63LL) * mode.height > INT_MAX) {
        fprintf(stderr, "[evdi-helper] Mode dimensions cannot be safely converted at scale %d\n", capture->scale);
        reject_mode(capture);
        return 0;
    }
    return 1;
}

static int retire_mode_buffers(capture_context_t *capture) {
    if (!frame_exchange_retire(capture->frames)) {
        fprintf(stderr, "[evdi-helper] Writer stuck during mode change — stopping capture\n");
        /* The writer may still hold the old size across its pacing sleep.
           Keep every buffer in place until shutdown joins it; replacing
           the exchange write buffer here would pair that old size with a new allocation. */
        capture->have_mode = 0;
        capture->capture_failed = 1;
        (*capture->running) = 0;
        return 0;
    }

    /* The kernel must drop its reference to the old framebuffer BEFORE we
       free it — freeing first is a use-after-free during a pending grab. */
    if (capture->buffer_registered) {
        evdi_unregister_buffer(capture->handle, 0);
        capture->buffer_registered = 0;
    }
    return 1;
}

static int configure_mode_geometry(capture_context_t *capture, struct evdi_mode mode) {
    int new_swap_rb = mode.pixel_format == 0x34324258 || mode.pixel_format == 0x34324241;
    int new_w = mode.width;
    int new_h = mode.height;
    int new_bpp = mode.bits_per_pixel / 8;
    if (new_bpp < 1) new_bpp = 4;

    /* Same geometry (KWin re-applying the mode)? Keep everything as-is.
       Re-registering on every event is what crashed libevdi before. */
    if (capture->buffer_registered && new_w == capture->mode_w && new_h == capture->mode_h
            && new_bpp == capture->mode_bpp && new_swap_rb == capture->swap_rb) {
        fprintf(stderr, "[evdi-helper] Mode unchanged, keeping buffer\n");
        capture->update_pending = 0;
        return 0;
    }

    if (!retire_mode_buffers(capture)) return 0;

    capture->swap_rb = new_swap_rb;
    capture->cursor.order = capture_pixel_order(capture);
    capture->mode_w = new_w;
    capture->mode_h = new_h;
    capture->mode_bpp = new_bpp;

    int row_bytes = capture->mode_w * capture->mode_bpp;
    int aligned_stride = (row_bytes + 63) & ~63;  /* DRM buffers are 64-byte aligned */
    capture->mode_stride = aligned_stride;
    capture->fb_size = capture->mode_stride * capture->mode_h;

    return 1;
}

static void allocate_framebuffer(capture_context_t *capture) {
    capture->cursor.drawn = (cursor_rect_t){0};
    free(capture->framebuffer);
    free(capture->native_framebuffer);
    capture->native_framebuffer = NULL;
    if (capture->swap_rb && capture->normalize_rgba) {
        capture->native_framebuffer = malloc(capture->fb_size);
        if (capture->native_framebuffer) memset(capture->native_framebuffer, 0x18, capture->fb_size);
    }
    /* Page-aligned, prefaulted capture storage. MADV_HUGEPAGE is best-effort:
       accepting the advice does not prove huge-page backing or remove the
       kernel-to-userspace capture copy. Measure actual backing and faults
       before attributing a speedup to page size; retain ordinary allocation
       when aligned allocation is unavailable. */
    {
        size_t huge = 2u << 20;
        size_t len = ((size_t)capture->fb_size + huge - 1) / huge * huge;
        void *p = NULL;
        if (posix_memalign(&p, huge, len) == 0 && p) {
            madvise(p, len, MADV_HUGEPAGE);
            memset(p, 0, len);
            capture->framebuffer = p;
        } else {
            capture->framebuffer = malloc(capture->fb_size);
        }
    }

}

static void allocate_stream_buffers(capture_context_t *capture) {
    /* Stream dimensions: source divided by the scale, forced even because
       NV12 chroma covers 2x2 luma samples. */
    int width = (capture->mode_w / capture->scale) & ~1;
    int height = (capture->mode_h / capture->scale) & ~1;
    frame_exchange_dimensions(capture->frames, width, height);
    printf("STREAM_SIZE %d %d\n", width, height);
    fflush(stdout);

    if (capture->raw_ring) {
        if (!raw_ring_resize(capture->raw_ring, width, height))
            reject_mode(capture);
    } else {
        frame_exchange_resize(capture->frames, width, height);
    }

}

static int mode_buffers_allocated(capture_context_t *capture) {
    if (capture->swap_rb && capture->normalize_rgba && !capture->native_framebuffer) return 0;
    if (capture->raw_ring) return capture->framebuffer && !capture->capture_failed;
    return capture->framebuffer && frame_exchange_allocated(capture->frames);
}

static void on_mode_changed(struct evdi_mode mode, void *user_data) {
    capture_context_t *capture = user_data;
    if (capture->capture_failed) return;
    fprintf(stderr, "[evdi-helper] Mode: %dx%d@%dHz %dbpp fmt=0x%x\n",
            mode.width, mode.height, mode.refresh_rate,
            mode.bits_per_pixel, mode.pixel_format);
    if (!validate_frame_format(capture, mode) || !validate_mode_dimensions(capture, mode)) return;
    printf("MODE_CHANGED %d %d %d\n", mode.width, mode.height, mode.refresh_rate);
    fflush(stdout);

    if (!configure_mode_geometry(capture, mode)) return;

    allocate_framebuffer(capture);
    allocate_stream_buffers(capture);

    if (!mode_buffers_allocated(capture)) {
        fprintf(stderr, "[evdi-helper] Failed to allocate framebuffers\n");
        reject_mode(capture);
        return;
    }

    /* Dark gray initial frame so the tablet shows something immediately */
    memset(capture->framebuffer, 0x18, capture->fb_size);

    struct evdi_buffer buf = {
        .id = 0,
        .buffer = capture->native_framebuffer ? capture->native_framebuffer : capture->framebuffer,
        .width = capture->mode_w,
        .height = capture->mode_h,
        .stride = capture->mode_stride,
        .rects = NULL,
        .rect_count = 0,
    };
    evdi_register_buffer(capture->handle, buf);
    capture->buffer_registered = 1;

    frame_exchange_set_ready(capture->frames, 1);
    draw_cursor(capture);
    publish_frame(capture);

    capture->have_mode = 1;
    capture->update_pending = 0;
    fprintf(stderr, "[evdi-helper] Buffer 0 registered: %dx%d stride=%d (row_bytes=%d)\n",
            capture->mode_w, capture->mode_h, buf.stride, capture->mode_w * capture->mode_bpp);
}

static void normalize_framebuffer_rect(capture_context_t *capture, struct evdi_rect rect) {
    pixel_span_t x = pixel_span_clipped(rect.x1, rect.x2, capture->mode_w);
    pixel_span_t y = pixel_span_clipped(rect.y1, rect.y2, capture->mode_h);
    if (x.begin == x.end) return;
    for (int row = y.begin; row < y.end; row++) {
        size_t offset = (size_t)row * capture->mode_stride + (size_t)x.begin * 4;
        conv_copy_rgba_to_bgra(capture->framebuffer + offset,
            capture->native_framebuffer + offset, (size_t)(x.end - x.begin));
    }
}

/* Bound repeated swizzles by a full-frame pass without allocating a union.
 * The native framebuffer retains unchanged pixels, so conservative copies are safe.
 * Subtract clipped areas from a budget to avoid accumulation overflow. */
static int normalize_full_frame(const capture_context_t *capture, const struct evdi_rect *rects, int count) {
    if (count >= 64) return 1;
    size_t remaining = (size_t)capture->mode_w * capture->mode_h;
    for (int i = 0; i < count; i++) {
        pixel_span_t x = pixel_span_clipped(rects[i].x1, rects[i].x2, capture->mode_w);
        pixel_span_t y = pixel_span_clipped(rects[i].y1, rects[i].y2, capture->mode_h);
        size_t pixels = (size_t)(x.end - x.begin) * (y.end - y.begin);
        if (pixels >= remaining) return 1;
        remaining -= pixels;
    }
    return 0;
}

static void normalize_framebuffer(capture_context_t *capture, const struct evdi_rect *rects, int count) {
    if (!capture->swap_rb || !capture->normalize_rgba || count <= 0) return;
    if (normalize_full_frame(capture, rects, count)) {
        normalize_framebuffer_rect(capture, (struct evdi_rect){0, 0, capture->mode_w, capture->mode_h});
        return;
    }
    for (int i = 0; i < count; i++) normalize_framebuffer_rect(capture, rects[i]);
}

static void grab_now(capture_context_t *capture)
{
    if (!capture_active(capture))
        return;
    struct evdi_rect rects[64];
    int num_rects = 64;
    restore_cursor(capture);
    long long t0 = capture_now_us();
    evdi_grab_pixels(capture->handle, rects, &num_rects);
    long long grabbed_us = capture_now_us();
    capture->grab_sum_us += grabbed_us - t0;
    normalize_framebuffer(capture, rects, num_rects);
    draw_cursor(capture);
    capture->grab_n++;
    if (num_rects <= 0)
        capture->empty_n++;
    if (num_rects > 0)
    {
        capture->grab_count++;
        capture->grab_us = grabbed_us;
        /* Under the swap lock: the writer thread reassigns the mask pointers
           when it takes a frame, so touching them unlocked would race.
           If the driver returned more rectangles than we gave it room for,
           we cannot know what else changed — repaint everything. */
        if (num_rects >= (int)(sizeof(rects) / sizeof(rects[0])))
            mark_all_dirty(capture);
        else
            mark_damage(capture, rects, num_rects);
        publish_frame(capture);
    }
}

static void on_update_ready(int buffer_to_be_updated, void *user_data)
{
    capture_context_t *capture = user_data;
    (void)buffer_to_be_updated;
    if (capture->capture_failed || !capture_active(capture))
        return;
    if (capture->update_pending)
    {
        capture->wait_sum_us += capture_now_us() - capture->req_us;
        capture->wait_n++;
    }
    capture->update_pending = 0;
    grab_now(capture);
    /* GRABPIX acknowledges the pending flip before any subsequent request
       (T599). Keep the request's deadline: an early ready callback must not
       bypass the configured rate (T750). Overdue requests remain immediate. */
}

static void on_crtc_state(int state, void *user_data) {
    (void)user_data;
    fprintf(stderr, "[evdi-helper] CRTC state: %d\n", state);
}

static int cursor_event_image(struct evdi_cursor_set event, cursor_image_t *image) {
    switch (event.pixel_format) {
    case 0x34325241: image->alpha = 1; break; /* AR24 */
    case 0x34325258: break; /* XR24 */
    case 0x34324241: image->alpha = 1; image->swap_rb = 1; break; /* AB24 */
    case 0x34324258: image->swap_rb = 1; break; /* XB24 */
    default: return 0;
    }
    image->pixels = (const unsigned char *)event.buffer;
    image->length = event.buffer_length;
    image->width = event.width; image->height = event.height; image->stride = event.stride;
    return 1;
}

static void publish_cursor(capture_context_t *capture) {
    draw_cursor(capture);
    capture->grab_us = capture_now_us();
    publish_frame(capture);
}

static void on_cursor_set(struct evdi_cursor_set event, void *user_data) {
    capture_context_t *capture = user_data;
    if (capture->capture_failed) {
        free(event.buffer);
        return;
    }
    restore_cursor(capture);
    cursor_image_t image = {0};
    int valid = !event.enabled || cursor_event_image(event, &image);
    if (valid && event.enabled) {
        valid = cursor_overlay_take(&capture->cursor, &image);
        event.buffer = NULL;
    } else if (valid) valid = cursor_overlay_set(&capture->cursor, NULL);
    /* libevdi transfers its allocated cursor image to this callback. */
    free(event.buffer);
    if (!valid) {
        fprintf(stderr, "[evdi-helper] Invalid or unavailable cursor image; stopping capture\n");
        reject_mode(capture);
        return;
    }
    publish_cursor(capture);
}

static void on_cursor_move(struct evdi_cursor_move event, void *user_data) {
    capture_context_t *capture = user_data;
    if (capture->capture_failed) return;
    restore_cursor(capture);
    /* EVDI events already report the plane top-left; do not subtract hotspot. */
    capture->cursor.x = event.x; capture->cursor.y = event.y;
    publish_cursor(capture);
}

static long capture_period_ms(capture_context_t *capture) {
    /* Round up: 16ms requests would permit 62.5fps for a configured60fps.
       Keep arithmetic wide for malformed/extreme inputs. The same deadline
       applies to immediate and asynchronous request completion. */
    long long fps = capture->fps > 0 ? capture->fps : 60;
    return (long)((1000 + fps - 1) / fps);
}

static int capture_poll_timeout(capture_context_t *capture, long request_period_ms)
{
    if (!capture_active(capture))
        return 100;
    /* Sleep exactly until the next capture request is due instead of a
       fixed 4ms tick. The fixed tick quantised every request to a 4ms grid,
       adding up to 4ms of jitter per frame — a quarter of the entire budget
       at 60fps, and half of it at 120.
       With no mode there is nothing to request, and the deadline below
       would sit permanently in the past — poll would return instantly and
       the loop would spin at 100% of a core. Wait on events only. */
    long long timeout_ms;
    if (!capture->have_mode)
    {
        timeout_ms = 100;
    }
    else if (capture->update_pending)
    {
        /* Waiting for update_ready. The capture deadline below has already
           passed and cannot advance until the request completes, so
           deriving a timeout from it yields 0 forever and poll returns
           instantly — the loop then burns a whole core. This is not a
           rare state: disabling the virtual output leaves a mode set with
           nothing rendering to it, which is exactly what happens when the
           tablet is unplugged or switched to pen-only.
           Sleep until the watchdog is due instead. update_ready wakes poll
           the moment it arrives, so nothing is delayed when the
           compositor is actually running. */
        long long left = capture->last_request_ms + 250 - capture_now_ms();
        timeout_ms = left < 0 ? 0 : (left > 250 ? 250 : (int)left);
    }
    else
    {
        long long due = capture->last_request_ms + request_period_ms;
        /* Keep overdue deadlines wide until bounded, including a first
           request after weeks of system uptime. */
        timeout_ms = due - capture_now_ms();
        if (timeout_ms < 0)
            timeout_ms = 0;
        if (timeout_ms > INT_MAX)
            timeout_ms = INT_MAX;
    }

    return (int)timeout_ms;
}

static void request_capture_if_due(capture_context_t *capture, evdi_handle handle, long long now, long request_period_ms) {
    /* Core capture cycle: request a fresh frame from the compositor at
       the target fps. If the kernel says pixels are ready right away,
       grab immediately; otherwise update_ready will fire and grab. */
    if (!capture->update_pending && (now - capture->last_request_ms) >= request_period_ms) {
        capture->last_request_ms = now;
        capture->req_us = capture_now_us();
        if (evdi_request_update(handle, 0)) {
            capture->immediate_n++;
            grab_now(capture);
        } else {
            capture->update_pending = 1;
        }
    }

}

static void recover_capture_if_stalled(capture_context_t *capture, long long now, long long *last_fallback_grab_ms) {
    /* Watchdog: if a request got lost (compositor hiccup), don't stay
       stuck waiting for update_ready forever. */
    if (capture->update_pending && (now - capture->last_request_ms) >= 250) {
        capture->update_pending = 0;
        grab_now(capture);
        /* This recovery also satisfies the fallback deadline. A second grab
         * on the same tick has no intervening update and only repeats work. */
        (*last_fallback_grab_ms) = now;
    }

    /* Fallback grab once a second in case no events flow at all */
    if ((now - (*last_fallback_grab_ms)) >= 1000) {
        (*last_fallback_grab_ms) = now;
        if (!capture->update_pending)
            grab_now(capture);
    }

}

static void report_capture_stats(capture_context_t *capture, long long now, long long *last_stats_ms, long long *stats_grab_base) {
    double elapsed = (now - (*last_stats_ms)) / 1000.0;
    long long grabs = capture->grab_count - (*stats_grab_base);
    fprintf(stderr, "[evdi-helper] %.1f grabs/s (total %lld), mode:%d dpms:%d pending:%d\n",
            elapsed > 0 ? grabs / elapsed : 0,
            capture->grab_count, capture->have_mode, capture->dpms_on, capture->update_pending);
    fprintf(stderr, "[evdi-helper] cycle: request→ready avg %.1fms (%d waited, %d immediate), grab avg %.1fms (%d, %d empty)\n",
            capture->wait_n ? capture->wait_sum_us / 1000.0 / capture->wait_n : 0.0, capture->wait_n, capture->immediate_n,
            capture->grab_n ? capture->grab_sum_us / 1000.0 / capture->grab_n : 0.0, capture->grab_n, capture->empty_n);
    capture->wait_sum_us = 0; capture->wait_n = 0; capture->grab_sum_us = 0; capture->grab_n = 0;
    capture->immediate_n = 0; capture->empty_n = 0;

    /* Capture-side latency: grab → convert → into the encoder's FIFO. */
    int snapshot[LAT_SAMPLES];
    int n = (int)frame_exchange_latency_take(capture->frames, snapshot, LAT_SAMPLES);
    if (n > 0) {
        qsort(snapshot, (size_t)n, sizeof(int), cmp_int);
        fprintf(stderr,
                "[evdi-helper] capture→fifo p50 %.1fms p95 %.1fms (%d frames)\n",
                snapshot[n / 2] / 1000.0,
                snapshot[(int)((n - 1) * 0.95)] / 1000.0, n);
    }

    (*stats_grab_base) = capture->grab_count;
    (*last_stats_ms) = now;
}

static int capture_wakeup_fd(const capture_context_t *capture) {
    if (!capture) return -1;
    if (capture->raw_ring) return capture->raw_ring->socket;
    return frame_exchange_wakeup_fd(capture->frames);
}

static void resume_capture(capture_context_t *capture)
{
    if (!capture->have_mode || capture->capture_failed)
        return;
    mark_all_dirty(capture);
    frame_exchange_set_ready(capture->frames, 1);
    capture->update_pending = 0;
    capture->last_request_ms = capture_now_ms();
    capture->raw_sent_us = 0;
    long long previous = capture->grab_count;
    /* A queued flip must be acknowledged before any new update request (T599).
     * Even an unchanged image must seed the new encoder's first full frame. */
    grab_now(capture);
    capture->last_fallback_grab_ms = capture_now_ms();
    if (capture->grab_count == previous)
        publish_frame(capture);
}

static void apply_capture_activity(capture_context_t *capture)
{
    if (capture_active(capture))
        resume_capture(capture);
    else
    {
        /* Invalidate exactly once. A timeout preserves all original buffers;
         * later iterations observe completion without manufacturing a lease. */
        frame_exchange_retire(capture->frames);
        capture->raw_pending = 0;
    }
}

static int acknowledge_capture_activity(capture_context_t *capture)
{
    if (!capture_active(capture) && !frame_exchange_quiescent(capture->frames))
        return 1;
    return capture_control_ack(capture->control) >= 0;
}

static int service_capture_control(capture_context_t *capture)
{
    if (!capture || !capture->control)
        return 1;
    int changed = capture_control_poll(capture->control);
    if (changed < 0)
        return 0;
    if (changed)
        apply_capture_activity(capture);
    return acknowledge_capture_activity(capture);
}

static int capture_event_count(capture_context_t *capture, struct pollfd *fds)
{
    if (capture && capture->control)
    {
        capture_control_t *control = capture->control;
        fds[2] = (struct pollfd){.fd = -1, .events = POLLIN};
        if (control->current.sequence == control->acknowledged)
            fds[2].fd = control->fd;
        return 3;
    }
    return capture_wakeup_fd(capture) >= 0 ? 2 : 1;
}

static void dispatch_capture_events(evdi_handle handle, struct evdi_event_context *events,
                                    struct pollfd *fds, int ready)
{
    if (ready > 0 && (fds->revents & POLLIN))
        evdi_handle_events(handle, events);
    capture_context_t *capture = events->user_data;
    if (capture && frame_exchange_take_request(capture->frames))
        publish_frame(capture);
}

static int poll_capture_events(evdi_handle handle, struct evdi_event_context *evtctx,
                               struct pollfd *fd, int timeout_ms)
{
    capture_context_t *capture = evtctx->user_data;
    if (!service_capture_control(capture))
        return -1;
    int count = capture_event_count(capture, fd);
    int ret = poll(fd, count, timeout_ms);
    if (ret < 0)
    {
        if (errno == EINTR)
            return 0;
        fprintf(stderr, "[evdi-helper] poll() error: %s\n", strerror(errno));
        return -1;
    }
    if (fd->revents & (POLLHUP | POLLERR | POLLNVAL))
    {
        fprintf(stderr, "[evdi-helper] Event channel failed (poll flags 0x%x)\n", fd->revents);
        return -1;
    }
    /* Apply pause before dispatching a simultaneously queued pixel callback. */
    if (!service_capture_control(capture))
        return -1;
    dispatch_capture_events(handle, evtctx, fd, ret);
    return 1;
}

static int publish_shared_capture(capture_context_t *capture)
{
    if (!capture_active(capture))
        return 1;
    if (!capture->have_mode || !frame_exchange_is_ready(capture->frames))
        return 1;
    long long now = capture_now_us();
    long long interval = capture->raw_pending ? 1000000 / capture->fps : 200000;
    if (now - capture->raw_sent_us < interval)
        return 1;
    uint32_t slot;
    unsigned char *pixels = raw_ring_acquire(capture->raw_ring, &slot);
    if (!pixels)
        return 1; /* Keep latest unencoded update pending while all slots are leased. */
    raw_ring_t *ring = capture->raw_ring;
    capture_to_nv12(capture, capture->framebuffer, pixels, ring->dirty[slot], ring->spans[slot]);
    raw_ring_converted(ring, slot);
    int result = raw_ring_publish(capture->raw_ring, slot, capture->grab_us);
    if (result < 0)
        return 0;
    if (result > 0)
    {
        capture->raw_sent_us = now;
        capture->raw_pending = 0;
    }
    return 1;
}

static int service_shared_capture(capture_context_t *capture) {
    if (!capture->raw_ring) return 1;
    return raw_ring_service(capture->raw_ring) && publish_shared_capture(capture);
}

static int shared_poll_timeout(capture_context_t *capture, int timeout_ms)
{
    if (!capture_active(capture))
        return timeout_ms;
    if (!capture->raw_ring || !capture->have_mode)
        return timeout_ms;
    long long interval = capture->raw_pending ? 1000000 / capture->fps : 200000;
    long long remaining = capture->raw_sent_us + interval - capture_now_us();
    /* A full ring must not spin while libavcodec retains all slots. Release
     * notifications wake poll immediately; 1 ms also covers a lost hint. */
    int due = remaining <= 0 ? 1 : (int)((remaining + 999) / 1000);
    return due < timeout_ms ? due : timeout_ms;
}

static int capture_tick(capture_context_t *capture, evdi_handle handle, long request_period_ms,
                        long long *last_stats_ms, long long *stats_grab_base)
{
    if (!capture_active(capture))
        return service_shared_capture(capture);
    if (!capture->have_mode)
        return 1;
    long long now = capture_now_ms();
    request_capture_if_due(capture, handle, now, request_period_ms);
    recover_capture_if_stalled(capture, now, &capture->last_fallback_grab_ms);
    if (now - *last_stats_ms >= 5000)
        report_capture_stats(capture, now, last_stats_ms, stats_grab_base);
    return service_shared_capture(capture);
}

int capture_run(capture_context_t *capture, evdi_handle handle)
{
    evdi_enable_cursor_events(handle, true);
    struct evdi_event_context evtctx = {
        .dpms_handler = on_dpms,
        .mode_changed_handler = on_mode_changed,
        .update_ready_handler = on_update_ready,
        .crtc_state_handler = on_crtc_state,
        .cursor_set_handler = on_cursor_set,
        .cursor_move_handler = on_cursor_move,
        .user_data = capture,
    };

    struct pollfd fds[3] = {0};
    fds[0].fd = evdi_get_event_ready(handle);
    fds[0].events = POLLIN;
    fds[1].fd = capture_wakeup_fd(capture);
    fds[1].events = POLLIN;

    long long last_stats_ms = capture_now_ms();
    long long stats_grab_base = 0;
    long request_period_ms = capture_period_ms(capture);

    while ((*capture->running))
    {
        int timeout_ms = capture_poll_timeout(capture, request_period_ms);
        timeout_ms = shared_poll_timeout(capture, timeout_ms);

        int ret = poll_capture_events(handle, &evtctx, fds, timeout_ms);
        if (ret < 0)
            return 1;
        if (ret == 0)
            continue;
        if (!service_shared_capture(capture))
            return 1;

        if (!capture_tick(capture, handle, request_period_ms, &last_stats_ms, &stats_grab_base))
            return 1;
    }
    return capture->capture_failed;
}
