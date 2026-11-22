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

static int t1692_expected_rank(size_t count, unsigned percentile)
{
    if (count == 0 || count > 256 || percentile == 0 || percentile > 100)
        return -1;
    for (size_t rank = 0; rank < count; rank++)
    {
        if ((rank + 1) * 100 >= count * percentile)
            return (int)rank;
    }
    assert(0 && "T1692 invalid independent quantile oracle");
    return -1;
}

static void test_t1692_ranks(void)
{
    for (size_t count = 0; count <= 258; count++)
    {
        int previous = -1;
        for (unsigned percentile = 0; percentile <= 101; percentile++)
        {
            int rank = latency_rank(count, percentile);
            assert(rank == t1692_expected_rank(count, percentile));
            if (rank >= 0)
            {
                assert(rank >= previous && rank < (int)count);
                previous = rank;
            }
        }
    }
    assert(latency_rank(SIZE_MAX, 50) == -1);
    assert(latency_rank(1, UINT_MAX) == -1);
}
