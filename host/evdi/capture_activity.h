#ifndef BLENT_CAPTURE_ACTIVITY_H
#define BLENT_CAPTURE_ACTIVITY_H
#include <stddef.h>
#include <stdint.h>

#define CAPTURE_ACTIVITY_BYTES 9

/* Independent wire contract: nonzero big-endian sequence, strict boolean.
 * Echo only after completing the exact transition. Sequences never wrap. */
typedef struct
{
    uint64_t sequence;
    int active;
} capture_activity_t;

int capture_activity_decode(const unsigned char *bytes, size_t length, capture_activity_t *request);
void capture_activity_encode(capture_activity_t request,
                             unsigned char bytes[CAPTURE_ACTIVITY_BYTES]);
int capture_activity_next(capture_activity_t request, uint64_t previous);
#endif
