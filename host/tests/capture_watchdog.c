/* T1607: an empty recovery must not repeat at the initial 250 ms cadence. */
static void t1607_empty_watchdog(void)
{
    g_capture.have_mode = 1;
    mock_grab_calls = 0;
    t623_request_calls = 0;
    long long fallback = 1000;
    mock_monotonic_ms = 1000;
    request_capture_if_due(&g_capture, g_capture.handle, 1000, 16);
    mock_monotonic_ms = 1250;
    recover_capture_if_stalled(&g_capture, 1250, &fallback);
    assert(mock_grab_calls == 1);
    request_capture_if_due(&g_capture, g_capture.handle, 1250, 16);
    mock_monotonic_ms = 1500;
    assert(capture_poll_timeout(&g_capture, 16) == 250);
    recover_capture_if_stalled(&g_capture, 1500, &fallback);
    assert(mock_grab_calls == 1 && "T1607 empty capture recovery did not back off");
    mock_monotonic_ms = 1750;
    recover_capture_if_stalled(&g_capture, 1750, &fallback);
    assert(mock_grab_calls == 2);
    request_capture_if_due(&g_capture, g_capture.handle, 1750, 16);
    mock_monotonic_ms = 2250;
    assert(capture_poll_timeout(&g_capture, 16) == 500);
    recover_capture_if_stalled(&g_capture, 2250, &fallback);
    assert(mock_grab_calls == 2);
    mock_monotonic_ms = 2750;
    recover_capture_if_stalled(&g_capture, 2750, &fallback);
    assert(mock_grab_calls == 3);
    mock_grab_calls = -1;
    mock_monotonic_ms = -1;
    t623_request_calls = -1;
}

static void t1607_recovery_bounds(void)
{
    const int values[] = {INT_MIN, -1, 0, 249, 250, 500, 1000, 1001, INT_MAX};
    const int expected[] = {250, 250, 250, 250, 250, 500, 1000, 1000, 1000};
    for (unsigned i = 0; i < sizeof(values) / sizeof(values[0]); i++)
    {
        g_capture.watchdog_ms = values[i];
        assert(watchdog_delay(&g_capture) == expected[i]);
    }
    mock_grab_calls = 0;
    t623_request_calls = 0;
    mock_monotonic_ms = 3000;
    long long fallback = 3000;
    for (int step = 0; step < 60; step++)
    {
        request_capture_if_due(&g_capture, g_capture.handle, mock_monotonic_ms, 16);
        mock_monotonic_ms += 999;
        recover_capture_if_stalled(&g_capture, mock_monotonic_ms, &fallback);
        assert(mock_grab_calls == step);
        mock_monotonic_ms++;
        recover_capture_if_stalled(&g_capture, mock_monotonic_ms, &fallback);
        assert(mock_grab_calls == step + 1);
        assert(watchdog_delay(&g_capture) == 1000);
    }
    on_update_ready(0, &g_capture);
    assert(watchdog_delay(&g_capture) == 250);
    g_capture.watchdog_ms = 1000;
    t623_ready = 1;
    request_capture_if_due(&g_capture, g_capture.handle, mock_monotonic_ms, 16);
    assert(watchdog_delay(&g_capture) == 250);
    t623_ready = 0;
    t623_request_calls = -1;
    mock_grab_calls = -1;
    mock_monotonic_ms = -1;
}

static void t1607_nonempty_recovery(void)
{
    frame_exchange_init(&g_frames);
    on_mode_changed((struct evdi_mode){8, 8, 60, 32, 0x34325258}, &g_capture);
    t752_grabs = 1;
    t752_count = 1;
    t752_rects[0] = (struct evdi_rect){0, 0, 8, 8};
    g_capture.watchdog_ms = 1000;
    recover_pending_update(&g_capture);
    assert(watchdog_delay(&g_capture) == 250);
    t752_grabs = 0;
    t497_free_capture();
}

static void test_t1607_empty_watchdog(void)
{
    t1607_empty_watchdog();
    t1607_recovery_bounds();
    t1607_nonempty_recovery();
}
