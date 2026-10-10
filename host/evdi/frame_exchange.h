#ifndef BLENT_FRAME_EXCHANGE_H
#define BLENT_FRAME_EXCHANGE_H
#include <stdatomic.h>
#include <stddef.h>
#include <time.h>
#include "pixel_span.h"

#define LAT_SAMPLES 256
typedef struct frame_exchange frame_exchange_t;
typedef struct frame_damage frame_damage_t;

typedef struct { int have_frame; unsigned generation; } frame_cursor_t;
typedef struct {
    const unsigned char *data;
    size_t size;
    unsigned generation;
    long long grabbed_us;
    int fresh;
} frame_lease_t;

/* Capture-owner snapshot. Borrowed pointers remain valid through conversion,
 * until that same owner publishes/resizes; no pixel storage is copied. */
typedef struct {
    unsigned char *data;
    const unsigned char *dirty;
    const pixel_row_t *spans;
    int width, height;
} frame_target_t;
frame_target_t frame_exchange_target(const frame_exchange_t *frames);
/* Capture owner only; positive/even, size-safe dimensions validated by mode
 * owner. Mode metadata also describes the shared-ring path. */
void frame_exchange_dimensions(frame_exchange_t *frames, int width, int height);
int frame_exchange_is_ready(const frame_exchange_t *frames);
int frame_exchange_wakeup_fd(const frame_exchange_t *frames);
void frame_exchange_set_ready(frame_exchange_t *frames, int ready);
void frame_exchange_wake(frame_exchange_t *frames);
/* One atomic damage transaction. Callback must not block or retain the sink,
 * and must only call frame_damage_rect with this borrowed token. */
typedef void (*frame_damage_update_t)(frame_damage_t *damage, void *context);
void frame_exchange_update(frame_exchange_t *frames, frame_damage_update_t update, void *context);
void frame_damage_rect(frame_damage_t *damage, int x0, int y0, int x1, int y1, int scale);
void frame_exchange_invalidate(frame_exchange_t *frames);
void frame_exchange_latency_add(frame_exchange_t *frames, long long micros);
/* Drain at most capacity samples; excess samples remain for the next call. */
size_t frame_exchange_latency_take(frame_exchange_t *frames, int *samples, size_t capacity);

void frame_exchange_init(frame_exchange_t *frames);
/* FIFO only, before writer startup. Failure leaves legacy conversion enabled. */
int frame_exchange_enable_demand(frame_exchange_t *frames);
/* Writer only, without an active lease. Invalidates cached frames on transitions. */
void frame_exchange_reader(frame_exchange_t *frames, int connected);
/* Capture owner: drain the wakeup hint and check whether a full refresh is due. */
int frame_exchange_take_request(frame_exchange_t *frames);
/* Capture owner: snapshot demand/generation before converting; zero skips work. */
int frame_exchange_begin(frame_exchange_t *frames, unsigned *generation);
/* Invalidate generation, then wait boundedly for the writer's last lease.
 * On failure every old allocation must survive until the writer is joined. */
int frame_exchange_retire(frame_exchange_t *frames);
/* Capture thread only: retire() must have succeeded; dimensions are validated
 * positive/even and size-safe by the mode owner before this call. */
void frame_exchange_resize(frame_exchange_t *frames, int width, int height);
int frame_exchange_allocated(const frame_exchange_t *frames);
/* Publish only if the conversion still belongs to this reader/mode generation. */
void frame_exchange_publish(frame_exchange_t *frames, long long grabbed_us, unsigned generation);
/* Absolute CLOCK_MONOTONIC keepalive deadline, or NULL to wait for an event.
 * -1 stopped, 0 no frame, 1 immutable lease, always followed by release(). */
int frame_exchange_claim(frame_exchange_t *frames, frame_cursor_t *cursor,
                         const atomic_int *running, const struct timespec *deadline, frame_lease_t *lease);
/* Owns locking; call without holding mutex. Notifies retirement waiters. */
void frame_exchange_release(frame_exchange_t *frames);
/* Only after writer join and conversion completion. */
void frame_exchange_free(frame_exchange_t *frames);
#endif
