#include <sys/resource.h>

/* T1120: independent native script; no allocator-dependent inode reuse gamble.
 * The preexisting request intentionally matches the new source descriptor. */
static struct
{
    const char *advice_path;
    int publish_current;
    int wait_steps;
} t1120_script;

static void t1120_path(char *path, size_t capacity, const char *leaf)
{
    const char *root = getenv("BLENT_T1120_ROOT");
    assert(root);
    int count = snprintf(path, capacity, "%s/%s", root, leaf);
    assert(count > 0);
    assert((size_t)count < capacity);
}

static void t1120_new_request(void)
{
    char replacement[PATH_MAX];
    t1120_path(replacement, sizeof(replacement), "current-XXXXXX");
    int file = mkstemp(replacement);
    assert(file >= 0);
    assert(close(file) == 0);
    t1119_request(replacement, g_fifo.fd, 2700, 1000);
    assert(rename(replacement, t1120_script.advice_path) == 0);
}

static int t1120_wait(const struct timespec *deadline)
{
    assert(deadline->tv_sec == 1);
    assert(deadline->tv_nsec == 200000000L * (t1120_script.wait_steps + 1));
    t1120_script.wait_steps++;
    mock_monotonic_ms = 1000 + 200 * t1120_script.wait_steps;
    if (t1120_script.wait_steps == 1)
    {
        if (t1120_script.publish_current)
            t1120_new_request();
    }
    else
    {
        assert(t1120_script.wait_steps == 2);
        g_running = 0;
    }
    return ETIMEDOUT;
}

static void t1120_writer_fixture(int publish_current, int unsafe_advice)
{
    char path[PATH_MAX];
    t1120_path(path, sizeof(path), "old-XXXXXX");
    int file = mkstemp(path);
    assert(file >= 0);
    assert(close(file) == 0);
    int ends[2];
    assert(pipe(ends) == 0);
    g_fifo.fd = ends[1];
    g_writer.idle_control = path;
    t1119_request(path, ends[1], 2500, 500);
    assert(idle_control_interval(path, ends[1], 1000) == 500);
    struct stat original;
    assert(stat(path, &original) == 0);
    if (unsafe_advice)
        assert(chmod(path, 0666) == 0);
    frame_exchange_init(&g_frames);
    frame_exchange_resize(&g_frames, 4, 4);
    assert(frame_exchange_allocated(&g_frames));
    memset(g_frames.fill, 0x41, (size_t)g_frames.size);
    g_frames.buffers_ready = 1;
    frame_exchange_publish(&g_frames, 0, g_frames.generation);
    t1120_script.wait_steps = 0;
    t1120_script.advice_path = path;
    t1120_script.publish_current = publish_current;
    t1120_wait_observer = t1120_wait;
    g_running = 1;
    mock_monotonic_ms = 1000;
    writer_run(&g_writer);
    t1120_wait_observer = NULL;
    assert(t1120_script.wait_steps == 2);
    assert(!g_frames.writer_busy);
    unsigned char received[48];
    ssize_t expected = publish_current ? 24 : 48;
    assert(read(ends[0], received, sizeof(received)) == expected &&
           "T1120: old advice controls a replacement source, or new advice was ignored");
    struct stat retained;
    assert(stat(path, &retained) == 0);
    assert((original.st_ino != retained.st_ino) == publish_current);
    frame_exchange_free(&g_frames);
    assert(pthread_cond_destroy(&g_frames.ready) == 0);
    assert(unlink(path) == 0);
    assert(close(ends[0]) == 0);
    assert(close(ends[1]) == 0);
    g_fifo.fd = -1;
    g_writer.idle_control = NULL;
    t1120_script.advice_path = NULL;
    mock_monotonic_ms = -1;
}

static void test_t1120_stale_advice(void)
{
    t1120_writer_fixture(0, 0);
}

static void test_t1120_current_advice(void)
{
    t1120_writer_fixture(1, 0);
}

static void test_t1120_reader_lifetime(void)
{
    char path[PATH_MAX];
    t1120_path(path, sizeof(path), "pin-XXXXXX");
    int file = mkstemp(path);
    assert(file >= 0);
    assert(close(file) == 0);
    int ends[2];
    assert(pipe(ends) == 0);
    t1119_request(path, ends[1], 2500, 500);
    idle_control_reader_t reader = IDLE_CONTROL_READER_INITIALIZER;
    assert(idle_control_begin(&reader, path));
    int original_fd = reader.fd;
    struct stat original;
    assert(fstat(original_fd, &original) == 0);
    assert(fcntl(original_fd, F_GETFD) & FD_CLOEXEC);
    assert(fcntl(original_fd, F_GETFL) & O_PATH);
    assert(!idle_control_begin(&reader, path));
    assert(reader.fd == original_fd);
    assert(idle_control_reader_interval(&reader, path, ends[1], 1000) == 0);
    assert(unlink(path) == 0);
    t1119_request(path, ends[1], 2500, 1000);
    assert(idle_control_reader_interval(&reader, path, ends[1], 1000) == 1000);
    struct stat pinned;
    assert(fstat(original_fd, &pinned) == 0);
    assert(pinned.st_dev == original.st_dev);
    assert(pinned.st_ino == original.st_ino);
    struct stat current;
    assert(stat(path, &current) == 0);
    assert(current.st_ino != original.st_ino);
    idle_control_end(&reader);
    assert(fcntl(original_fd, F_GETFD) == -1);
    assert(errno == EBADF);
    /* The next real open reuses the released descriptor. A repeated end must
     * preserve its new owner, and a retired reader cannot query it. */
    int replacement = open(path, O_PATH | O_CLOEXEC);
    assert(replacement == original_fd);
    idle_control_end(&reader);
    assert(fstat(replacement, &pinned) == 0);
    assert(pinned.st_ino == current.st_ino);
    assert(idle_control_reader_interval(&reader, path, ends[1], 1000) == 0);
    assert(close(replacement) == 0);
    assert(unlink(path) == 0);
    assert(close(ends[0]) == 0);
    assert(close(ends[1]) == 0);
}

static void t1120_unknown_advice(const char *path)
{
    idle_control_reader_t reader = IDLE_CONTROL_READER_INITIALIZER;
    assert(!idle_control_begin(&reader, path));
    assert(!reader.admitted);
    assert(reader.fd == -1);
    assert(idle_control_reader_interval(&reader, path, -1, 1000) == 0);
    idle_control_end(&reader);
}

static void test_t1120_reader_boundaries(void)
{
    char path[PATH_MAX];
    t1120_path(path, sizeof(path), "boundary-XXXXXX");
    int file = mkstemp(path);
    assert(file >= 0);
    assert(close(file) == 0);
    assert(chmod(path, 0666) == 0);
    t1120_unknown_advice(path);
    assert(chmod(path, 0600) == 0);
    char data[160] = {0};
    t1121_write(path, data, sizeof(data));
    t1120_unknown_advice(path);
    assert(unlink(path) == 0);
    assert(symlink("not-followed", path) == 0);
    t1120_unknown_advice(path);
    assert(unlink(path) == 0);
    assert(mkdir(path, 0700) == 0);
    t1120_unknown_advice(path);
    assert(rmdir(path) == 0);
    idle_control_reader_t reader = IDLE_CONTROL_READER_INITIALIZER;
    assert(idle_control_begin(&reader, path));
    assert(reader.fd == -1);
    idle_control_end(&reader);
    assert(idle_control_begin(&reader, NULL));
    idle_control_end(&reader);
    assert(advice_current(-1, NULL));
    struct stat current = {0};
    assert(advice_current(-9, &current)); /* Negative means no prior pin. */
    assert(!advice_current(INT_MAX, &current));
    /* ENOTDIR is an actual open error, independent of effective UID. */
    file = open(path, O_CREAT | O_WRONLY | O_CLOEXEC, 0600);
    assert(file >= 0);
    assert(close(file) == 0);
    char child[PATH_MAX];
    int count = snprintf(child, sizeof(child), "%s/child", path);
    assert(count > 0);
    assert((size_t)count < sizeof(child));
    t1120_unknown_advice(child);
    assert(unlink(path) == 0);
}

static void test_t1120_reader_exhaustion(void)
{
    char path[PATH_MAX];
    t1120_path(path, sizeof(path), "exhaustion");
    int file = open(path, O_CREAT | O_WRONLY | O_CLOEXEC, 0600);
    assert(file >= 0);
    assert(close(file) == 0);
    struct rlimit original;
    assert(getrlimit(RLIMIT_NOFILE, &original) == 0);
    struct rlimit exhausted = original;
    exhausted.rlim_cur = 0;
    idle_control_reader_t reader = IDLE_CONTROL_READER_INITIALIZER;
    assert(setrlimit(RLIMIT_NOFILE, &exhausted) == 0);
    int admitted = idle_control_begin(&reader, path);
    int failure = errno;
    assert(setrlimit(RLIMIT_NOFILE, &original) == 0);
    assert(!admitted);
    assert(failure == EMFILE);
    assert(!reader.admitted);
    assert(reader.fd == -1);
    idle_control_end(&reader);
    assert(unlink(path) == 0);
    idle_control_reader_t zero = {0};
    struct stat before, after;
    assert(fstat(STDIN_FILENO, &before) == 0);
    idle_control_end(&zero);
    assert(fstat(STDIN_FILENO, &after) == 0);
    assert(before.st_dev == after.st_dev);
    assert(before.st_ino == after.st_ino);
}

static void test_t1120_unsafe_writer_advice(void)
{
    t1120_writer_fixture(0, 1);
}
