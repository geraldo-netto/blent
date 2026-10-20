/* T1185: independent deferred-flip obligation, exercised through the actual
 * capture loop. The test owns initialized frames/control/handle until loop
 * return, and settles its immutable writer lease before freeing buffers. */
static struct
{
    evdi_handle handle;
    const unsigned char *native;
    unsigned pending;
    unsigned settled;
    unsigned completions;
    int grabs;
    long long last_grab;
} t1185_kernel;

static int t1185_poll(struct pollfd *fds, nfds_t count, int timeout)
{
    assert(count == 3 && timeout >= 0);
    assert(fds[0].fd == t1185_kernel.handle->fd);
    mock_monotonic_ms += 25;
    if (mock_monotonic_ms >= 1250)
        g_running = 0;
    return 0;
}

static void t1185_grab(evdi_handle handle, struct evdi_rect *rects, int *count)
{
    (void)rects;
    assert(handle == t1185_kernel.handle && *count == 64);
    assert(g_capture.framebuffer == t1185_kernel.native && g_capture.buffer_registered);
    assert(mock_monotonic_ms - t1185_kernel.last_grab >= 100);
    assert(g_running && "T1185: no native admission after stop cutoff");
    t1185_kernel.last_grab = mock_monotonic_ms;
    t1185_kernel.grabs++;
    if (t1185_kernel.pending)
    {
        t1185_kernel.settled = t1185_kernel.pending;
        t1185_kernel.pending = 0;
        t1185_kernel.completions++;
    }
    *count = 0;
}

static frame_lease_t t1185_writer_lease(capture_control_t *control)
{
    control->current.active = 1;
    publish_frame(&g_capture);
    control->current.active = 0;
    frame_cursor_t cursor = {0};
    frame_lease_t lease;
    assert(frame_exchange_claim(&g_frames, &cursor, &g_running, NULL, &lease) == 1);
    g_conversion.last_jobs = 0;
    return lease;
}

static void test_t1185_progress(void)
{
    capture_control_t control;
    int peer = t1021_pair(&control);
    t1021_mode(&control);
    struct evdi_device_context handle = {.fd = eventfd(0, EFD_NONBLOCK | EFD_CLOEXEC)};
    assert(handle.fd >= 0);
    g_capture.handle = &handle;
    frame_lease_t lease = t1185_writer_lease(&control);
    assert(lease.size == 96);
    unsigned char held[96];
    memcpy(held, lease.data, sizeof(held));
    t1185_kernel.handle = &handle;
    t1185_kernel.native = g_capture.framebuffer;
    t1185_kernel.pending = 42;
    t1185_kernel.last_grab = 1000;
    mock_monotonic_ms = 1000;
    t1185_poll_observer = t1185_poll;
    t1185_grab_observer = t1185_grab;
    assert(capture_run(&g_capture, &handle) == 0);
    int completed = t1185_kernel.settled == 42 && t1185_kernel.pending == 0;
    assert(t1185_kernel.completions <= 1);
    assert(!memcmp(held, lease.data, sizeof(held)));
    assert(g_conversion.last_jobs == 0 && !mock_disconnect_calls);
    assert(g_capture.grab_count == 0 && t623_request_calls == 0);
    t1185_poll_observer = NULL;
    t1185_grab_observer = NULL;
    mock_monotonic_ms = -1;
    frame_exchange_release(&g_frames);
    close(handle.fd);
    close(peer);
    close(control.fd);
    g_capture.control = NULL;
    t497_free_capture();
    assert(completed && "T1185: paused retained monitor stranded original DRM flip");
    assert(t1185_kernel.grabs == 2 && "T1185: native drain must remain paced");
}

static void test_t1185_resume_snapshot(void)
{
    capture_control_t control;
    int peer = t1021_pair(&control);
    g_capture.normalize_rgba = 1;
    t1021_mode(&control);
    /* AB24 allocates the separate native/canonical pair. */
    on_mode_changed((struct evdi_mode){8, 8, 60, 32, 0x34324241}, &g_capture);
    assert(g_capture.native_framebuffer && g_capture.framebuffer);
    memset(g_capture.native_framebuffer, 0x37, (size_t)g_capture.fb_size);
    memset(g_capture.framebuffer, 0x18, (size_t)g_capture.fb_size);
    t1021_send(peer, 1, 1);
    t1021_poll(&control);
    t1021_ack(peer, 1, 1);
    assert(g_capture.fb_size == 512 && g_capture.mode_stride == 64);
    int fresh = 1;
    for (int row = 0; row < 8; row++)
    {
        for (int column = 0; column < 64; column++)
        {
            unsigned char expected = column < 32 ? 0x37 : 0x18;
            if (g_capture.framebuffer[row * 64 + column] != expected)
                fresh = 0;
        }
    }
    close(peer);
    close(control.fd);
    g_capture.control = NULL;
    t497_free_capture();
    assert(fresh && "T1185: resume must normalize native pixels drained during pause");
}
