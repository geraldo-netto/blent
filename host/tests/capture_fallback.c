/* T1688: the capture event-loop owns the deadline; no new resource is retained. */
static void test_t1688_regular_grabs(void)
{
    g_capture.have_mode = 1;
    mock_grab_calls = 0;
    mock_monotonic_ms = 1000;
    g_capture.cadence.last_fallback_grab_ms = 1000;
    for (int step = 1; step <= 240; step++)
    {
        mock_monotonic_ms = 1000 + step * 17;
        on_update_ready(0, &g_capture);
        recover_capture_if_stalled(&g_capture, mock_monotonic_ms, &g_capture.cadence.last_fallback_grab_ms);
        assert(mock_grab_calls == step && "T1688 extra fallback during ordinary ready events");
    }
    assert(g_capture.cadence.last_fallback_grab_ms == mock_monotonic_ms);
    mock_monotonic_ms += 999;
    recover_capture_if_stalled(&g_capture, mock_monotonic_ms, &g_capture.cadence.last_fallback_grab_ms);
    assert(mock_grab_calls == 240);
    mock_monotonic_ms++;
    recover_capture_if_stalled(&g_capture, mock_monotonic_ms, &g_capture.cadence.last_fallback_grab_ms);
    assert(mock_grab_calls == 241);
    assert(g_capture.cadence.last_fallback_grab_ms == mock_monotonic_ms);
    mock_grab_calls = -1;
    mock_monotonic_ms = -1;
}
