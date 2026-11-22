/* T1689: independently count native requests over a half-open ten-second window. */
static void t1689_window(int fps)
{
    g_capture.fps = fps;
    g_capture.have_mode = 1;
    g_capture.update_pending = 0;
    g_capture.next_request_ns = 0;
    mock_grab_calls = 0;
    t623_request_calls = 0;
    t623_ready = 1;
    long long period = capture_period_ns(&g_capture);
    for (int elapsed = 0; elapsed < 10000; elapsed++)
    {
        mock_monotonic_ms = 1000 + elapsed;
        request_capture_if_due(&g_capture, g_capture.handle, mock_monotonic_ms * 1000000, period);
    }
    fprintf(stderr, "T1689 fps=%d requests=%d window_ms=10000\n", fps, t623_request_calls);
    assert(t623_request_calls == fps * 10 && "T1689 request cadence drifted below configured FPS");
    assert(mock_grab_calls == t623_request_calls);
    t623_ready = 0;
    mock_grab_calls = -1;
    t623_request_calls = -1;
    mock_monotonic_ms = -1;
}

static void test_t1689_sixty(void)
{
    t1689_window(60);
}

static void test_t1689_ninety(void)
{
    t1689_window(90);
}

static void t1689_late_phase(int fps)
{
    g_capture.fps = fps;
    g_capture.next_request_ns = 0;
    long long period = capture_period_ns(&g_capture);
    long long now = 1000000000;
    schedule_next_capture(&g_capture, now, period);
    for (int step = 0; step < 100; step++)
    {
        now = g_capture.next_request_ns + step * 1234567LL;
        schedule_next_capture(&g_capture, now, period);
        assert(g_capture.next_request_ns > now);
        assert(g_capture.next_request_ns <= now + period);
        assert((g_capture.next_request_ns - 1000000000) % period == 0);
    }
}

static void t1689_poll_rounding(void)
{
    mock_monotonic_ns = 1000000000;
    const long long offsets[] = {-1, 0, 1, 999999, 1000000, 1000001,
                                (long long)INT_MAX * 1000000,
                                ((long long)INT_MAX + 1) * 1000000};
    const int expected[] = {0, 0, 1, 1, 1, 2, INT_MAX, INT_MAX};
    for (unsigned i = 0; i < sizeof(offsets) / sizeof(offsets[0]); i++)
    {
        g_capture.next_request_ns = mock_monotonic_ns + offsets[i];
        assert(capture_request_wait_ms(&g_capture) == expected[i]);
    }
    mock_monotonic_ns = -1;
}

static void t1689_no_catchup_burst(void)
{
    g_capture.have_mode = 1;
    g_capture.update_pending = 0;
    g_capture.next_request_ns = 1000000000;
    mock_monotonic_ns = 100000000000;
    mock_grab_calls = 0;
    t623_request_calls = 0;
    t623_ready = 1;
    for (int retry = 0; retry < 100; retry++)
        request_capture_if_due(&g_capture, g_capture.handle, mock_monotonic_ns, 16666667);
    assert(t623_request_calls == 1 && mock_grab_calls == 1);
    mock_monotonic_ns = -1;
    mock_grab_calls = -1;
    t623_request_calls = -1;
    t623_ready = 0;
}

static void test_t1689_boundaries(void)
{
    for (int fps = 1; fps <= 240; fps++)
        t1689_late_phase(fps);
    t1689_poll_rounding();
    t1689_no_catchup_burst();
}
