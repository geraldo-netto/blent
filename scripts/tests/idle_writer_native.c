/* T1113: bounded original-generation/borrow checks; no EVDI or device I/O. */
#define _GNU_SOURCE
#include "frame_exchange_private.h"
#include <assert.h>

static int withdraw_on_target;

static frame_target_t observed_target(frame_exchange_t *frames)
{
    frame_target_t target = frame_exchange_target(frames);
    if (withdraw_on_target)
    {
        withdraw_on_target = 0;
        frame_exchange_reader(frames, 0);
    }
    return target;
}

#define frame_exchange_target observed_target
#define main idle_writer_entry
#include "../benchmarks/idle_writer.c"
#undef main
#undef frame_exchange_target

static frame_lease_t claim(frame_exchange_t *frames, frame_cursor_t *cursor)
{
    atomic_int running = 1;
    struct timespec deadline;
    assert(clock_gettime(CLOCK_MONOTONIC, &deadline) == 0);
    frame_lease_t lease;
    assert(frame_exchange_claim(frames, cursor, &running, &deadline, &lease) == 1);
    assert(lease.fresh == 1);
    assert(lease.size == 6);
    return lease;
}

static void verify_pixels(frame_lease_t lease, unsigned marker)
{
    for (size_t index = 0; index < lease.size; index++)
    {
        unsigned expected = index < 4 ? marker : 128;
        assert(lease.data[index] == expected);
    }
}

static void withdrawal(frame_exchange_t *frames)
{
    withdraw_on_target = 1;
    publish(frames, 77);
    assert(withdraw_on_target == 0);
    atomic_int running = 1;
    frame_cursor_t cursor = {0};
    frame_lease_t lease;
    struct timespec deadline;
    assert(clock_gettime(CLOCK_MONOTONIC, &deadline) == 0);
    assert(frame_exchange_claim(frames, &cursor, &running, &deadline, &lease) == 0);
    /* Demand is absent: rejected publication must not even modify fill. */
    frame_target_t target = frame_exchange_target(frames);
    unsigned before = target.data[0];
    publish(frames, 99);
    assert(target.data[0] == before);
    frame_exchange_reader(frames, 1);
}

static int original_writer_exit(void)
{
    int output = open("/dev/null", O_WRONLY | O_CLOEXEC);
    assert(output >= 0);
    assert(dup2(output, STDOUT_FILENO) == STDOUT_FILENO);
    assert(close(output) == 0);
    char *arguments[] = {"idle-writer", "30", "0", NULL};
    return idle_writer_entry(3, arguments);
}

int main(void)
{
    frame_exchange_t frames = FRAME_EXCHANGE_INITIALIZER;
    frame_exchange_init(&frames);
    frame_exchange_resize(&frames, 2, 2);
    assert(frame_exchange_allocated(&frames));
    frame_exchange_set_ready(&frames, 1);
    assert(frame_exchange_enable_demand(&frames));
    frame_exchange_reader(&frames, 1);
    withdrawal(&frames);
    frame_cursor_t cursor = {0};
    for (unsigned marker = 0; marker < 256; marker++)
    {
        publish(&frames, marker);
        frame_lease_t lease = claim(&frames, &cursor);
        verify_pixels(lease, marker);
        frame_exchange_release(&frames);
    }
    publish(&frames, 42);
    frame_lease_t original = claim(&frames, &cursor);
    assert(!frame_exchange_retire(&frames));
    assert(!frame_exchange_quiescent(&frames));
    verify_pixels(original, 42);
    frame_exchange_release(&frames);
    assert(frame_exchange_quiescent(&frames));
    frame_exchange_free(&frames);
    assert(pthread_cond_destroy(&frames.ready) == 0);
    assert(pthread_mutex_destroy(&frames.mutex) == 0);
    return original_writer_exit();
}
