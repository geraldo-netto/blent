/* T750: ready callbacks acknowledge immediately, without resetting the rate cap. */
static void t750_async_case(int fps, int delay) {
    g_capture.fps = fps; g_capture.have_mode = 1; g_capture.update_pending = 0;
    g_capture.last_request_ms = 0;
    mock_monotonic_ms = 1000; mock_grab_calls = 0;
    t623_request_calls = 0; t623_ready = 0;
    long period = capture_period_ms(&g_capture);
    request_capture_if_due(&g_capture, g_capture.handle, 1000, period);
    assert(t623_request_calls == 1 && g_capture.update_pending);
    mock_monotonic_ms += delay;
    int waits = g_capture.wait_n;
    on_update_ready(0, &g_capture);
    assert(mock_grab_calls == 1 && !g_capture.update_pending && g_capture.wait_n == waits + 1);
    assert(g_capture.last_request_ms == 1000 && "T750: early ready callback erased the capture rate deadline");
    int remaining = delay < period ? (int)period - delay : 0;
    assert(capture_poll_timeout(&g_capture, period) == remaining);
    /* Duplicate/late readiness still gets GRAB acknowledgement; no second wait. */
    on_update_ready(0, &g_capture);
    assert(mock_grab_calls == 2 && g_capture.wait_n == waits + 1);
    request_capture_if_due(&g_capture, g_capture.handle, mock_monotonic_ms, period);
    assert(t623_request_calls == (remaining ? 1 : 2));
    mock_monotonic_ms += remaining;
    request_capture_if_due(&g_capture, g_capture.handle, mock_monotonic_ms, period);
    assert(t623_request_calls == 2 && g_capture.update_pending);
}

static void test_t750_async(void) {
    for (int fps = 1; fps <= 240; fps++) {
        g_capture.fps = fps;
        int period = (int)capture_period_ms(&g_capture);
        const int delays[] = {0, 1, period - 1, period, period + 1, 250};
        for (unsigned i = 0; i < sizeof(delays)/sizeof(delays[0]); i++) t750_async_case(fps, delays[i]);
    }
    mock_monotonic_ms = -1; mock_grab_calls = -1; t623_request_calls = -1;
}
