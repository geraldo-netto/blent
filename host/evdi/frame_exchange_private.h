#ifndef BLENT_FRAME_EXCHANGE_PRIVATE_H
#define BLENT_FRAME_EXCHANGE_PRIVATE_H
#include "frame_exchange.h"
#include <pthread.h>

/* Buffer, dirty-mask and two-span row histories always travel together. A row is valid
 * only while its row's dirty bit is set. The capture owner alone converts fill;
 * the writer may borrow write until release(). */
struct frame_exchange {
    pthread_mutex_t mutex;
    pthread_cond_t ready;
    unsigned char *fill, *latest, *write;
    unsigned char *dirty_fill, *dirty_latest, *dirty_write;
    pixel_row_t *spans_fill, *spans_latest, *spans_write;
    int width, height, size, chroma_rows, dirty_bytes;
    int buffers_ready, latest_valid;
    int demand_fd, reader_connected, refresh_needed;
    atomic_int writer_busy;
    atomic_uint generation;
    long long latest_grab_us, write_grab_us;
    int latency[LAT_SAMPLES], latency_count;
};
#define FRAME_EXCHANGE_INITIALIZER { .mutex = PTHREAD_MUTEX_INITIALIZER, .demand_fd = -1 }

struct frame_damage { frame_exchange_t *frames; };

/* Call damage routines with mutex held once a writer exists. */
void frame_exchange_mark_all(frame_exchange_t *frames);
void frame_exchange_damage(frame_exchange_t *frames, int y0, int y1, int scale);
/* Source rectangle; normalizes reversed endpoints, clips bounds and includes
 * complete scaled chroma blocks. Empty/off-frame rectangles do no work. */
void frame_exchange_damage_rect(frame_exchange_t *frames, int x0, int y0, int x1, int y1, int scale);
#endif
