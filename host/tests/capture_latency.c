/* T1690: bounded samples are not an all-frame latency distribution. */
static void test_t1690_sample_window(void)
{
    long long stats = 0;
    long long grabs = 0;
    report_capture_stats(&g_capture, 5000, &stats, &grabs);
    for (int sample = 0; sample < 300; sample++)
        frame_exchange_latency_add(&g_frames, sample * 1000);
    report_capture_stats(&g_capture, 10000, &stats, &grabs);
    assert(g_frames.latency_count == 0);
    frame_exchange_latency_add(&g_frames, 7000);
    frame_exchange_latency_add(&g_frames, 8000);
    report_capture_stats(&g_capture, 15000, &stats, &grabs);
    assert(g_frames.latency_count == 0);
}
