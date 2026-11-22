/* T1713: advice reads belong to idle decisions, not fresh frame admission. */
#define _GNU_SOURCE
#include <fcntl.h>
#include <stdarg.h>
#include <string.h>
#include <sys/types.h>
#include <unistd.h>
static const char *watched_path;
static int watched_fd = -1, advice_opens, advice_reads;
static int counted_open(const char *path, int flags, ...)
{
    mode_t mode = 0;
    if (flags & O_CREAT)
    {
        va_list args;
        va_start(args, flags);
        mode = va_arg(args, int);
        va_end(args);
    }
    int fd = open(path, flags, mode);
    if (watched_path && !strcmp(path, watched_path))
    {
        advice_opens++;
        watched_fd = fd;
    }
    return fd;
}

static ssize_t counted_read(int fd, void *buffer, size_t count)
{
    if (fd >= 0 && fd == watched_fd)
        advice_reads++;
    return read(fd, buffer, count);
}

static int counted_close(int fd)
{
    if (fd == watched_fd)
        watched_fd = -1;
    return close(fd);
}

#define open counted_open
#define read counted_read
#define close counted_close
#define BLENT_C_FUZZ_DRIVER
#include "evdi_helper_test.c"
#undef open
#undef read
#undef close
typedef struct
{
    char path[64];
    int source[2];
    writer_state_t state;
} advice_probe_t;

static long long expected_wait_ms = -1;
int __real_pthread_cond_timedwait(pthread_cond_t *, pthread_mutex_t *, const struct timespec *);

int __wrap_pthread_cond_timedwait(pthread_cond_t *condition, pthread_mutex_t *mutex,
                                  const struct timespec *deadline)
{
    if (expected_wait_ms < 0)
        return __real_pthread_cond_timedwait(condition, mutex, deadline);
    assert(deadline->tv_sec * 1000 + deadline->tv_nsec / 1000000 == expected_wait_ms);
    mock_monotonic_ms = expected_wait_ms;
    return ETIMEDOUT;
}

static void advice_begin(advice_probe_t *probe)
{
    strcpy(probe->path, "/tmp/blent-t1713-advice-XXXXXX");
    int file = mkstemp(probe->path);
    assert(file >= 0 && close(file) == 0);
    assert(pipe(probe->source) == 0);
    probe->state =
        (writer_state_t){.frame = {.generation = UINT_MAX}, .idle = IDLE_CONTROL_READER_INITIALIZER};
    assert(idle_control_begin(&probe->state.idle, probe->path));
    g_fifo.fd = probe->source[1];
    g_writer.idle_control = probe->path;
    frame_exchange_init(&g_frames);
    frame_exchange_resize(&g_frames, 4, 4);
    assert(frame_exchange_allocated(&g_frames));
    memset(g_frames.fill, 42, (size_t)g_frames.size);
    g_frames.buffers_ready = 1;
    g_running = 1;
    mock_monotonic_ms = 1000;
    t1119_request(probe->path, probe->source[1], 2500, 500);
    watched_path = probe->path;
}

static void fresh_frames(advice_probe_t *probe)
{
    for (int frame = 0; frame < 1000; frame++)
    {
        frame_exchange_publish(&g_frames, 0, g_frames.generation);
        int size, fresh;
        assert(claim_writer_frame(&g_writer, &probe->state, &size, &fresh) == 1 && fresh);
        assert(writer_frame_due(&g_writer, &probe->state, fresh));
        frame_exchange_release(&g_frames);
    }
    fprintf(stderr, "T1713 fresh_frames=1000 advice_opens=%d advice_reads=%d\n", advice_opens, advice_reads);
}

static void idle_decision(advice_probe_t *probe, long long deadline, int write)
{
    expected_wait_ms = deadline;
    int size, fresh;
    assert(claim_writer_frame(&g_writer, &probe->state, &size, &fresh) == 1 && !fresh);
    assert(writer_frame_due(&g_writer, &probe->state, fresh) == write);
    if (write)
        frame_exchange_release(&g_frames);
    assert(!g_frames.writer_busy);
    expected_wait_ms = -1;
}

static void idle_deadlines(advice_probe_t *probe)
{
    idle_decision(probe, 1200, 0);
    idle_decision(probe, 1400, 0);
    idle_decision(probe, 1500, 1);
    t1119_request(probe->path, probe->source[1], 1600, 500);
    idle_decision(probe, 1700, 1);
    assert(unlink(probe->path) == 0);
    idle_decision(probe, 1900, 1);
    t1119_request(probe->path, probe->source[1], 3500, 1000);
    for (long long deadline = 2100; deadline < 2900; deadline += 200)
        idle_decision(probe, deadline, 0);
    idle_decision(probe, 2900, 1);
}

static void advice_end(advice_probe_t *probe)
{
    watched_path = NULL;
    idle_control_end(&probe->state.idle);
    assert(frame_exchange_quiescent(&g_frames) == 0);
    assert(frame_exchange_retire(&g_frames));
    frame_exchange_free(&g_frames);
    pthread_cond_destroy(&g_frames.ready);
    pthread_mutex_destroy(&g_frames.mutex);
    g_writer.idle_control = NULL;
    g_fifo.fd = -1;
    close(probe->source[0]);
    close(probe->source[1]);
    assert(unlink(probe->path) == 0);
}

int main(void)
{
    advice_probe_t probe;
    advice_begin(&probe);
    fresh_frames(&probe);
    int valid = advice_opens == 0 && advice_reads == 0;
    advice_opens = advice_reads = 0;
    idle_deadlines(&probe);
    fprintf(stderr, "T1713 idle_decisions=10 advice_opens=%d advice_reads=%d\n", advice_opens, advice_reads);
    valid = valid && advice_opens == 10 && advice_reads == 9;
    advice_end(&probe);
    assert(valid && "T1713 advice reads must follow idle decisions only");
    return 0;
}
