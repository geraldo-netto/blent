#ifndef BLENT_CAPTURE_H
#define BLENT_CAPTURE_H
#include "evdi_lib.h"
#include "frame_exchange.h"
#include "conversion.h"
#include "raw_ring.h"
#include "cursor_overlay.h"
#include "capture_control.h"

/* Event-loop-owned diagnostic cadence. Reset on media pause; no resource,
 * callback or pixel borrow survives a synchronous diagnostic operation. */
typedef struct
{
    long long report_ms;
    int reported;
} capture_progress_trace_t;

/* Event-loop-owned mode, registered framebuffer and capture statistics.
 * EVDI callbacks borrow this context via user_data and never outlive run().
 * The exchange/pool/stop flag must outlive it; conversion completes before
 * publishing, and mode retirement waits for the writer before reallocating. */
typedef struct {
    cursor_overlay_t cursor;
    frame_exchange_t *frames;
    conv_pool_t *conversion;
    atomic_int *running;
    evdi_handle handle;
    int device_index;
    int capture_failed;
    capture_control_t *control; /* optional, borrowed for the whole capture loop */
    unsigned char * framebuffer; /* native pixels, or canonical BGRA fallback, with local cursor */
    unsigned char *native_framebuffer; /* owned only for canonical RGBA fallback */
    int swap_rb;
    int normalize_rgba; /* explicit canonical fallback; default uses native order */
    int fb_size;
    int mode_w;
    int mode_h;
    int mode_bpp;
    int mode_stride;
    int have_mode;
    int dpms_on;
    int scale;
    int fps;
    long long grab_us;
    int update_pending;
    long long grab_count;
    long long req_us;
    long long wait_sum_us;
    int wait_n;
    long long grab_sum_us;
    int grab_n;
    int immediate_n;
    int empty_n;
    long long last_request_ms;
    long long last_fallback_grab_ms; /* capture-owner pacing, including resume snapshots */
    capture_progress_trace_t display_progress_trace;
    int buffer_registered;
    raw_ring_t *raw_ring;
    int raw_pending;
    long long raw_sent_us;
} capture_context_t;
#define CAPTURE_INITIALIZER(exchange, pool, stop) { .frames = (exchange), \
    .conversion = (pool), .running = (stop), .handle = EVDI_INVALID_HANDLE, \
    .device_index = -1, .mode_bpp = 4, .scale = 1, .fps = 60 }

int capture_run(capture_context_t *capture, evdi_handle handle);
#endif
