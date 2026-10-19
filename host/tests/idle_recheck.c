/* T1119: native request validation and bounded rechecks; no EVDI device. */
static void t1119_request(const char *path, int source, long long expiry, int interval)
{
    struct stat identity;
    assert(fstat(source, &identity) == 0);
    FILE *file = fopen(path, "w");
    assert(file);
    assert(fprintf(file, "%ju %ju 1 %lld %d\n", (uintmax_t)identity.st_dev,
                   (uintmax_t)identity.st_ino, expiry, interval) > 0);
    assert(fclose(file) == 0);
}

static void test_t1119_interval(void)
{
    char path[] = "/tmp/blent-t1119-value-XXXXXX";
    int file = mkstemp(path);
    assert(file >= 0);
    assert(close(file) == 0);
    int ends[2];
    assert(pipe(ends) == 0);
    g_fifo.fd = ends[1];
    g_writer.idle_control = path;
    mock_monotonic_ms = 1000;
    frame_exchange_init(&g_frames);
    const int intervals[] = {500, 1000};
    for (unsigned index = 0; index < sizeof(intervals) / sizeof(intervals[0]); index++)
    {
        int interval = intervals[index];
        t1119_request(path, ends[1], 2500, interval);
        mock_monotonic_ms = 1000;
        assert(writer_idle_ms(&g_writer) == interval && "T1119: sparse advice rejected");
        writer_state_t state = {.last_write_ms = 1000};
        mock_monotonic_ms = 1000 + interval - 1;
        assert(!writer_frame_due(&g_writer, &state, 0));
        mock_monotonic_ms++;
        assert(writer_frame_due(&g_writer, &state, 0));
        assert(writer_frame_due(&g_writer, &state, 1));
    }
    assert(pthread_cond_destroy(&g_frames.ready) == 0);
    mock_monotonic_ms = 2500;
    assert(writer_idle_ms(&g_writer) == 200);
    assert(unlink(path) == 0);
    assert(writer_idle_ms(&g_writer) == 200);
    assert(close(ends[0]) == 0);
    assert(close(ends[1]) == 0);
    g_fifo.fd = -1;
    g_writer.idle_control = NULL;
    mock_monotonic_ms = -1;
}

static void t1119_recheck(int interval)
{
    char path[] = "/tmp/blent-t1119-wait-XXXXXX";
    int file = mkstemp(path);
    assert(file >= 0);
    assert(close(file) == 0);
    int ends[2];
    assert(pipe(ends) == 0);
    g_fifo.fd = ends[1];
    g_writer.idle_control = path;
    t1119_request(path, ends[1], 2500, interval);
    frame_exchange_init(&g_frames);
    frame_exchange_resize(&g_frames, 4, 4);
    assert(frame_exchange_allocated(&g_frames));
    memset(g_frames.fill, 42, (size_t)g_frames.size);
    g_frames.buffers_ready = 1;
    frame_exchange_publish(&g_frames, 0, g_frames.generation);
    g_running = 1;
    mock_monotonic_ms = 1000;
    /* Independent observer requires 1200 ms, not sparse 1500 ms. Its stop
     * ends the original synchronous writer before frame storage is freed. */
    t405_check_idle_wait = 1;
    writer_run(&g_writer);
    assert(t405_idle_wait_seen == 1);
    assert(!g_frames.writer_busy);
    unsigned char data[24];
    assert(read(ends[0], data, sizeof(data)) == sizeof(data));
    assert(data[0] == 42 && data[23] == 42);
    frame_exchange_free(&g_frames);
    assert(pthread_cond_destroy(&g_frames.ready) == 0);
    mock_monotonic_ms = -1;
    assert(unlink(path) == 0);
    assert(close(ends[0]) == 0);
    assert(close(ends[1]) == 0);
    g_fifo.fd = -1;
    g_writer.idle_control = NULL;
}

static void test_t1119_recheck(void)
{
    t1119_recheck(500);
}

static void test_t1119_one_second_recheck(void)
{
    t1119_recheck(1000);
}
