/* T1121: public native boundary consumes exactly the supplied record. */
static void t1121_write(const char *path, const char *text, size_t count)
{
    int file = open(path, O_WRONLY | O_TRUNC | O_CLOEXEC);
    assert(file >= 0);
    assert(write(file, text, count) == (ssize_t)count);
    assert(close(file) == 0);
}

static void t1121_hidden_suffix(const char *path, int source, const char *record, size_t count)
{
    char text[160];
    assert(count + 17 < sizeof(text));
    memcpy(text, record, count);
    text[count] = '\0';
    for (size_t extra = 1; extra <= 16; extra++)
    {
        for (unsigned value = 0; value <= 255; value++)
        {
            memset(text + count + 1, value, extra);
            t1121_write(path, text, count + 1 + extra);
            assert(idle_control_interval(path, source, 1000) == 0 &&
                   "T1121: embedded NUL hides trailing native control bytes");
        }
    }
}

static void t1121_prefix_mutations(const char *path, int source, const char *record, size_t count)
{
    char text[160];
    for (size_t length = 0; length < count; length++)
    {
        t1121_write(path, record, length);
        assert(idle_control_interval(path, source, 1000) == 0);
    }
    for (size_t position = 0; position < count; position++)
    {
        memcpy(text, record, count);
        text[position] = '\0';
        t1121_write(path, text, count);
        assert(idle_control_interval(path, source, 1000) == 0);
    }
}

static void test_t1121_exact_record(void)
{
    char path[] = "/tmp/blent-t1121-record-XXXXXX";
    int file = mkstemp(path);
    assert(file >= 0);
    assert(close(file) == 0);
    int ends[2];
    assert(pipe(ends) == 0);
    struct stat identity;
    assert(fstat(ends[1], &identity) == 0);
    const int intervals[] = {500, 1000};
    for (unsigned index = 0; index < sizeof(intervals) / sizeof(intervals[0]); index++)
    {
        char text[160];
        int count = snprintf(text, sizeof(text), "%ju %ju 1 2500 %d\n", (uintmax_t)identity.st_dev,
                             (uintmax_t)identity.st_ino, intervals[index]);
        assert(count > 0);
        assert((size_t)count < sizeof(text));
        t1121_write(path, text, (size_t)count);
        assert(idle_control_interval(path, ends[1], 1000) == intervals[index]);
        t1121_hidden_suffix(path, ends[1], text, (size_t)count);
        t1121_prefix_mutations(path, ends[1], text, (size_t)count);
    }
    assert(unlink(path) == 0);
    assert(close(ends[0]) == 0);
    assert(close(ends[1]) == 0);
}
