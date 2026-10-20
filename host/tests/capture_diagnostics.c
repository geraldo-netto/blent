/* T1186: observe original control/native boundaries without claiming that a
 * media ACK retires the retained DRM output. Hooks borrow this stack fixture;
 * original writer/control/buffers remain owned until explicit release/close. */
static int t1186_pending_writer(const struct timespec *deadline)
{
    (void)deadline;
    return ETIMEDOUT;
}

static void t1186_native_return(evdi_handle handle, struct evdi_rect *rects, int *count)
{
    (void)handle;
    (void)rects;
    fputs("T1186_NATIVE_RETURN\n", stderr);
    mock_monotonic_ms += 5;
    *count = 0;
}

static void test_t1186_trace(void)
{
    capture_control_t control;
    int peer = t1021_pair(&control);
    t1021_mode(&control);
    mock_monotonic_ms = 1000;
    t1021_send(peer, 1, 1);
    t1021_poll(&control);
    t1021_ack(peer, 1, 1);
    publish_frame(&g_capture);
    frame_cursor_t cursor = {0};
    frame_lease_t lease;
    assert(frame_exchange_claim(&g_frames, &cursor, &g_running, NULL, &lease) == 1);
    t1120_wait_observer = t1186_pending_writer;
    t1021_send(peer, 2, 0);
    t1021_poll(&control);
    t1021_poll(&control);
    assert(control.acknowledged == 1);
    t1185_grab_observer = t1186_native_return;
    const long long samples[] = {1100, 1200, 6000, 6100};
    for (size_t i = 0; i < sizeof(samples) / sizeof(samples[0]); i++)
    {
        mock_monotonic_ms = samples[i];
        service_display_progress(&g_capture, EVDI_INVALID_HANDLE);
    }
    fputs("T1186_WRITER_RELEASE\n", stderr);
    frame_exchange_release(&g_frames);
    t1021_poll(&control);
    t1021_ack(peer, 2, 0);
    t1021_poll(&control);
    t1021_send(peer, 3, 1);
    t1021_poll(&control);
    t1021_ack(peer, 3, 1);
    t1021_send(peer, 4, 0);
    t1021_poll(&control);
    t1021_ack(peer, 4, 0);
    mock_monotonic_ms = 6300;
    service_display_progress(&g_capture, EVDI_INVALID_HANDLE);
    t1185_grab_observer = NULL;
    t1120_wait_observer = NULL;
    mock_monotonic_ms = -1;
    close(peer);
    close(control.fd);
    g_capture.control = NULL;
    t497_free_capture();
}

static void t1186_admission_case(capture_control_t *control, unsigned flags)
{
    control->current.active = flags & 1;
    g_capture.have_mode = (flags >> 1) & 1;
    g_capture.capture_failed = (flags >> 2) & 1;
    g_capture.buffer_registered = (flags >> 3) & 1;
    g_running = (flags >> 4) & 1;
    g_capture.last_fallback_grab_ms = 1000;
    mock_monotonic_ms = 1100;
    int before = mock_grab_calls;
    service_display_progress(&g_capture, EVDI_INVALID_HANDLE);
    assert(mock_grab_calls - before == (flags == 26));
}

static void test_t1186_admission(void)
{
    capture_control_t control;
    int peer = t1021_pair(&control);
    t1021_mode(&control);
    control.current.sequence = UINT64_MAX;
    for (unsigned flags = 0; flags < 32; flags++)
        t1186_admission_case(&control, flags);
    control.current.active = 0;
    g_capture.have_mode = 1;
    g_capture.capture_failed = 0;
    g_capture.buffer_registered = 1;
    g_running = 1;
    mock_monotonic_ms = 1000;
    apply_capture_activity(&g_capture);
    int before = mock_grab_calls;
    for (int elapsed = 0; elapsed <= 1000; elapsed++)
    {
        mock_monotonic_ms = 1000 + elapsed;
        service_display_progress(&g_capture, EVDI_INVALID_HANDLE);
        assert(mock_grab_calls - before == elapsed / 100);
    }
    assert(g_capture.grab_count == 0 && t623_request_calls == 0);
    assert(g_conversion.last_jobs == 0 && !mock_disconnect_calls);
    mock_monotonic_ms = -1;
    close(peer);
    close(control.fd);
    g_capture.control = NULL;
    t497_free_capture();
}
