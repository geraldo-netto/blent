#include "capture_activity.h"

int capture_activity_decode(const unsigned char *bytes, size_t length, capture_activity_t *request)
{
    if (length != CAPTURE_ACTIVITY_BYTES || bytes[8] > 1)
        return 0;
    uint64_t sequence = 0;
    for (int i = 0; i < 8; i++)
        sequence = (sequence << 8) | bytes[i];
    if (!sequence)
        return 0;
    *request = (capture_activity_t){sequence, bytes[8]};
    return 1;
}

void capture_activity_encode(capture_activity_t request,
                             unsigned char bytes[CAPTURE_ACTIVITY_BYTES])
{
    for (int i = 0; i < 8; i++)
        bytes[7 - i] = request.sequence >> (i * 8);
    bytes[8] = request.active;
}

int capture_activity_next(capture_activity_t request, uint64_t previous)
{
    return previous != UINT64_MAX && request.sequence == previous + 1;
}
