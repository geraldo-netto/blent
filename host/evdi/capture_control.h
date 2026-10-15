#ifndef BLENT_CAPTURE_CONTROL_H
#define BLENT_CAPTURE_CONTROL_H
#include "capture_activity.h"

/* Linux event-loop-owned adapter. Initial state is paused. Ownership of fd
 * transfers on successful init; it must outlive capture and its writer. */
typedef struct
{
    int fd;
    capture_activity_t current;
    uint64_t acknowledged;
} capture_control_t;

int capture_control_init(capture_control_t *control, int fd);
/* 1 new request, 0 no request or original transition pending, -1 failure. */
int capture_control_poll(capture_control_t *control);
/* Caller proves transition complete before calling. A blocked send stays
 * pending; it never certifies a subsequent transition. 1 sent, 0 retry, -1 error. */
int capture_control_ack(capture_control_t *control);
#endif
