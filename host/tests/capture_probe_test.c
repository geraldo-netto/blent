/* T1712: count real descriptor probes in a private mocked-EVDI event loop. */
#define _GNU_SOURCE
#include <sys/socket.h>
#include <unistd.h>
static int tracked_control = -1, tracked_demand = -1, tracked_ring = -1;
static int control_reads, demand_reads, ring_reads;

static ssize_t counted_recvmsg(int fd, struct msghdr *message, int flags)
{
    if (fd == tracked_control)
        control_reads++;
    if (fd == tracked_ring)
        ring_reads++;
    return recvmsg(fd, message, flags);
}

static ssize_t counted_read(int fd, void *bytes, size_t count)
{
    if (fd == tracked_demand)
        demand_reads++;
    return read(fd, bytes, count);
}

#define recvmsg counted_recvmsg
#define read counted_read
#define BLENT_C_FUZZ_DRIVER
#include "evdi_helper_test.c"
#undef read
#undef recvmsg

typedef struct
{
    capture_control_t control;
    raw_ring_t ring;
    int control_peer, ring_peer;
} probe_t;

static void probe_ring(probe_t *probe)
{
    int pair[2];
    assert(socketpair(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK, 0, pair) == 0);
    probe->ring_peer = pair[1];
    assert(raw_ring_init(&probe->ring, pair[0]));
    g_capture.raw_ring = &probe->ring;
    tracked_ring = probe->ring.socket;
}

static void probe_mode(probe_t *probe)
{
    on_mode_changed((struct evdi_mode){8, 8, 60, 32, 0x34325258}, &g_capture);
    raw_ring_t hello = probe->ring;
    hello.nonce = 1712;
    t418_control(&hello, probe->ring_peer, 4);
    assert(raw_ring_service(&probe->ring));
    t418_drain(probe->ring_peer);
    t1021_send(probe->control_peer, 1, 1);
    assert(service_capture_control(&g_capture));
    t1021_ack(probe->control_peer, 1, 1);
    for (unsigned i = 0; i < RAW_SLOTS; i++)
    {
        g_capture.publication.sent_us = 0;
        assert(publish_shared_capture(&g_capture));
        t418_drain(probe->ring_peer);
    }
    assert(probe->ring.sequence == RAW_SLOTS);
}

static void probe_begin(probe_t *probe, int mode)
{
    g_frames = (frame_exchange_t)FRAME_EXCHANGE_INITIALIZER;
    g_conversion = (conv_pool_t)CONV_POOL_INITIALIZER;
    g_capture = (capture_context_t)CAPTURE_INITIALIZER(&g_frames, &g_conversion, &g_running);
    probe->ring = (raw_ring_t)RAW_RING_INITIALIZER;
    probe->ring_peer = -1;
    frame_exchange_init(&g_frames);
    probe->control_peer = t1021_pair(&probe->control);
    g_capture.control = &probe->control;
    tracked_control = probe->control.fd;
    mock_monotonic_ms = 1000;
    mock_grab_calls = 0;
    t623_request_calls = 0;
    if (mode)
        probe_ring(probe);
    else
    {
        assert(frame_exchange_enable_demand(&g_frames));
        tracked_demand = g_frames.demand_fd;
    }
    if (mode >= 2)
        probe_mode(probe);
    control_reads = demand_reads = ring_reads = mock_poll_calls = 0;
}

static void probe_tick(probe_t *probe, int frame)
{
    struct evdi_event_context events = {.user_data = &g_capture};
    struct pollfd fds[3] = {{.fd = -1}, {.fd = capture_wakeup_fd(&g_capture), .events = POLLIN}};
    if (frame)
    {
        atomic_store_explicit((_Atomic uint32_t *)probe->ring.memory, RAW_FREE, memory_order_release);
        mock_monotonic_ms += 17;
        g_capture.publication.pending = 1;
    }
    g_capture.cadence.next_request_ns = mock_monotonic_ms * 1000000 + 16666667;
    long long stats = mock_monotonic_ms, grabs = 0;
    assert(poll_capture_events(EVDI_INVALID_HANDLE, &events, fds, 0) == 1);
    assert(service_shared_capture(&g_capture, fds[1].revents));
    assert(capture_tick(&g_capture, EVDI_INVALID_HANDLE, 16666667, &stats, &grabs));
    if (probe->ring_peer >= 0)
        t418_drain(probe->ring_peer);
}

static void probe_end(probe_t *probe)
{
    assert(retire_mode_buffers(&g_capture));
    g_capture.control = NULL;
    g_capture.raw_ring = NULL;
    tracked_control = tracked_demand = tracked_ring = -1;
    raw_ring_close(&probe->ring);
    if (probe->ring_peer >= 0)
        close(probe->ring_peer);
    close(probe->control.fd);
    if (probe->control_peer >= 0)
        close(probe->control_peer);
    t497_free_capture();
    pthread_mutex_destroy(&g_frames.mutex);
}

static int probe_case(int mode)
{
    probe_t probe = {0};
    probe_begin(&probe, mode);
    for (int tick = 0; tick < 1000; tick++)
        probe_tick(&probe, mode == 3);
    fprintf(stderr,
            "T1712 mode=%d iterations=1000 control_recvmsg=%d demand_read=%d ring_recvmsg=%d polls=%d\n",
            mode, control_reads, demand_reads, ring_reads, mock_poll_calls);
    int valid = control_reads == 1000 && demand_reads == 0 && ring_reads == 0;
    if (mode == 2)
        assert(probe.ring.sequence == RAW_SLOTS);
    if (mode == 3)
        assert(probe.ring.sequence == RAW_SLOTS + 1000);
    probe_end(&probe);
    return valid;
}

static int late_pause_peer;

static int pause_after_poll(struct pollfd *fds, nfds_t count, int timeout)
{
    (void)timeout;
    for (nfds_t index = 0; index < count; index++)
        fds[index].revents = 0;
    t1021_send(late_pause_peer, 2, 0);
    return 0;
}

static void late_pause(void)
{
    probe_t probe = {0};
    probe_begin(&probe, 2);
    late_pause_peer = probe.control_peer;
    t1185_poll_observer = pause_after_poll;
    int grabs = mock_grab_calls;
    probe_tick(&probe, 0);
    t1185_poll_observer = NULL;
    assert(!capture_active(&g_capture));
    assert(mock_grab_calls == grabs);
    t1021_ack(probe.control_peer, 2, 0);
    probe_end(&probe);
}

static void ring_release(int hint)
{
    probe_t probe = {0};
    probe_begin(&probe, 2);
    atomic_store_explicit((_Atomic uint32_t *)probe.ring.memory, RAW_FREE, memory_order_release);
    g_capture.publication.sent_us = 0;
    if (hint)
        t418_control(&probe.ring, probe.ring_peer, 3);
    assert(shared_poll_timeout(&g_capture, 100) == 1);
    probe_tick(&probe, 0);
    assert(probe.ring.sequence == RAW_SLOTS + 1);
    assert(atomic_load_explicit((_Atomic uint32_t *)probe.ring.memory, memory_order_acquire) == RAW_READY);
    probe_end(&probe);
}

static void demand_saturation(void)
{
    probe_t probe = {0};
    probe_begin(&probe, 0);
    frame_exchange_reader(&g_frames, 1);
    uint64_t saturated = UINT64_MAX - 2;
    assert(write(g_frames.demand_fd, &saturated, sizeof(saturated)) == sizeof(saturated));
    frame_exchange_reader(&g_frames, 0);
    frame_exchange_reader(&g_frames, 1);
    probe_tick(&probe, 0);
    assert(demand_reads == 1 && g_frames.refresh_needed);
    on_mode_changed((struct evdi_mode){8, 8, 60, 32, 0x34325258}, &g_capture);
    t1021_send(probe.control_peer, 1, 1);
    probe_tick(&probe, 0);
    t1021_ack(probe.control_peer, 1, 1);
    assert(g_frames.latest_valid && !g_frames.refresh_needed);
    probe_end(&probe);
}

static void peer_loss(int raw)
{
    probe_t probe = {0};
    probe_begin(&probe, raw);
    if (raw)
    {
        close(probe.ring_peer);
        probe.ring_peer = -1;
    }
    else
    {
        close(probe.control_peer);
        probe.control_peer = -1;
    }
    struct evdi_event_context events = {.user_data = &g_capture};
    struct pollfd fds[3] = {{.fd = -1}, {.fd = capture_wakeup_fd(&g_capture), .events = POLLIN}};
    int status = poll_capture_events(EVDI_INVALID_HANDLE, &events, fds, 0);
    if (raw)
    {
        assert(status == 1 && (fds[1].revents & POLLHUP));
        assert(!service_shared_capture(&g_capture, fds[1].revents));
    }
    else
        assert(status == -1);
    probe_end(&probe);
}

int main(void)
{
    int valid = 1;
    for (int mode = 0; mode <= 3; mode++)
        if (!probe_case(mode))
            valid = 0;
    late_pause();
    ring_release(0);
    ring_release(1);
    demand_saturation();
    peer_loss(0);
    peer_loss(1);
    assert(valid && "T1712 quiet descriptors must not receive redundant probes");
    return 0;
}
