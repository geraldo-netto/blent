/* T1185 diagnostic, guest only. No production fix or regression pass claim.
 * Three immutable dumb buffers remain owned until helper retirement and CRTC
 * disable. atexit sponsors cleanup on each ordinary rejection/partial failure;
 * guest PID1 owns containment if the probe or original helper cannot retire. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
#include <xf86drm.h>
#include <xf86drmMode.h>

struct buffer
{
    uint32_t handle;
    uint32_t fb;
};

struct owner
{
    int fd;
    int control;
    pid_t helper;
    uint32_t crtc;
    uint32_t connector;
    drmModeModeInfo mode;
    int enabled;
    struct buffer buffers[3];
};

static struct owner original = {.fd = -1, .control = -1, .helper = -1};
static unsigned event_count;
static uintptr_t last_token;

static void require(int condition, const char *stage)
{
    if (!condition)
    {
        fprintf(stderr, "T1185_REJECT stage=%s errno=%d\n", stage, errno);
        exit(1);
    }
}

static long long now_ms(void)
{
    struct timespec value;
    require(clock_gettime(CLOCK_MONOTONIC, &value) == 0, "clock");
    return (long long)value.tv_sec * 1000 + value.tv_nsec / 1000000;
}

static void destroy_buffer(struct buffer *buffer)
{
    if (buffer->fb)
        drmModeRmFB(original.fd, buffer->fb);
    if (buffer->handle)
    {
        struct drm_mode_destroy_dumb destroy = {.handle = buffer->handle};
        drmIoctl(original.fd, DRM_IOCTL_MODE_DESTROY_DUMB, &destroy);
    }
}

static void retire_helper(void)
{
    if (original.helper <= 0)
        return;
    kill(original.helper, SIGTERM);
    int status = 0;
    pid_t result;
    do
    {
        result = waitpid(original.helper, &status, 0);
    } while (result < 0 && errno == EINTR);
    printf("T1185_HELPER_RETIRED pid=%d waited=%d status=%d\n", original.helper, result, status);
    if (result != original.helper)
    {
        /* No release/reuse claim without the original wait. Guest PID1
         * contains this failure and destroys the private VM resource domain. */
        fputs("T1185_UNSETTLED_HELPER\n", stderr);
        _exit(76);
    }
    original.helper = -1;
}

static void cleanup(void)
{
    retire_helper();
    if (original.control >= 0)
        close(original.control);
    if (original.fd < 0)
        return;
    if (original.enabled)
        drmModeSetCrtc(original.fd, original.crtc, 0, 0, 0, NULL, 0, NULL);
    for (size_t i = 0; i < 3; i++)
        destroy_buffer(&original.buffers[i]);
    close(original.fd);
    puts("T1185_DRM_OWNER_CLOSED");
}

static void require_guest(void)
{
    char bytes[4096] = {0};
    FILE *file = fopen("/proc/cmdline", "r");
    require(file != NULL, "guest command line");
    int read = fgets(bytes, sizeof(bytes), file) != NULL;
    fclose(file);
    require(read && strstr(bytes, "blent-evdi-review=1"), "guest marker");
}

static void exec_helper(int fd)
{
    char number[32];
    snprintf(number, sizeof(number), "%d", fd);
    int log = open("/helper.log", O_WRONLY | O_CREAT | O_TRUNC, 0600);
    if (log < 0)
        _exit(126);
    dup2(log, STDOUT_FILENO);
    dup2(log, STDERR_FILENO);
    close(log);
    execl("/evdi_helper", "evdi_helper", "--edid", "/edid.bin", "--capture-fifo", "/tmp/video",
          "--capture-activity-fd", number, "--conversion-threads", "1", NULL);
    _exit(127);
}

static void start_helper(void)
{
    int pair[2];
    require(socketpair(AF_UNIX, SOCK_SEQPACKET, 0, pair) == 0, "control socket");
    original.control = pair[0];
    original.helper = fork();
    if (original.helper == 0)
    {
        close(pair[0]);
        exec_helper(pair[1]);
    }
    close(pair[1]);
    require(original.helper > 0, "helper fork");
}

static void control(unsigned sequence, int active)
{
    unsigned char bytes[9] = {0};
    bytes[7] = sequence;
    bytes[8] = active;
    require(send(original.control, bytes, sizeof(bytes), MSG_NOSIGNAL) == sizeof(bytes), "request");
    struct pollfd pending = {.fd = original.control, .events = POLLIN};
    require(poll(&pending, 1, 5000) == 1, "control observation window");
    unsigned char reply[10];
    require(recv(original.control, reply, sizeof(reply), 0) == sizeof(bytes), "ACK extent");
    require(memcmp(bytes, reply, sizeof(bytes)) == 0, "original ACK identity");
    printf("T1185_ACK sequence=%u active=%d at_ms=%lld\n", sequence, active, now_ms());
}

static drmModeConnectorPtr connected(drmModeResPtr resources)
{
    for (;;)
    {
        drmModeConnectorPtr connector = drmModeGetConnector(original.fd, resources->connectors[0]);
        require(connector != NULL, "connector");
        if (connector->connection == DRM_MODE_CONNECTED && connector->count_modes > 0)
            return connector;
        drmModeFreeConnector(connector);
        siginfo_t status = {0};
        require(waitid(P_PID, original.helper, &status, WEXITED | WNOHANG | WNOWAIT) == 0,
                "helper observation");
        require(status.si_pid == 0, "helper remains live");
        usleep(10000);
    }
}

static void create_buffer(struct buffer *buffer, unsigned color)
{
    struct drm_mode_create_dumb dumb = {.width = 640, .height = 480, .bpp = 32};
    require(drmIoctl(original.fd, DRM_IOCTL_MODE_CREATE_DUMB, &dumb) == 0, "allocate dumb");
    buffer->handle = dumb.handle;
    struct drm_mode_map_dumb map = {.handle = dumb.handle};
    require(drmIoctl(original.fd, DRM_IOCTL_MODE_MAP_DUMB, &map) == 0, "map offset");
    uint32_t *pixels =
        mmap(NULL, dumb.size, PROT_READ | PROT_WRITE, MAP_SHARED, original.fd, map.offset);
    require(pixels != MAP_FAILED, "map dumb");
    for (uint64_t i = 0; i < dumb.size / sizeof(*pixels); i++)
        pixels[i] = color;
    require(munmap(pixels, dumb.size) == 0, "unmap initialized dumb");
    require(drmModeAddFB(original.fd, 640, 480, 24, 32, dumb.pitch, dumb.handle, &buffer->fb) == 0,
            "framebuffer");
}

static void start_scanout(void)
{
    original.fd = open("/dev/dri/card0", O_RDWR | O_CLOEXEC);
    require(original.fd >= 0, "DRM open");
    drmVersionPtr version = drmGetVersion(original.fd);
    require(version != NULL, "DRM identity");
    int evdi = strcmp(version->name, "evdi") == 0;
    printf("T1185_DRIVER name=%s version=%d.%d.%d\n", version->name, version->version_major,
           version->version_minor, version->version_patchlevel);
    drmFreeVersion(version);
    require(evdi, "EVDI identity");
    drmModeResPtr resources = drmModeGetResources(original.fd);
    require(resources != NULL, "DRM resources");
    require(resources->count_crtcs == 1 && resources->count_connectors == 1, "single output");
    drmModeConnectorPtr connector = connected(resources);
    original.crtc = resources->crtcs[0];
    original.connector = connector->connector_id;
    original.mode = connector->modes[0];
    for (size_t i = 0; i < 3; i++)
        create_buffer(&original.buffers[i], 0xff000000u + (unsigned)i * 0x00404040u);
    int result = drmModeSetCrtc(original.fd, original.crtc, original.buffers[0].fb, 0, 0,
                                &connector->connector_id, 1, &connector->modes[0]);
    original.enabled = result == 0;
    drmModeFreeConnector(connector);
    drmModeFreeResources(resources);
    require(result == 0, "initial modeset");
}

static void flip_received(int fd, unsigned sequence, unsigned sec, unsigned usec, void *token)
{
    (void)fd;
    (void)sequence;
    (void)sec;
    (void)usec;
    event_count++;
    last_token = (uintptr_t)token;
    printf("T1185_FLIP token=%lu count=%u at_ms=%lld\n", (unsigned long)last_token, event_count,
           now_ms());
}

static int observe_event(int window_ms)
{
    struct pollfd pending = {.fd = original.fd, .events = POLLIN};
    int ready = poll(&pending, 1, window_ms);
    require(ready >= 0, "DRM poll");
    if (!ready)
        return 0;
    require(pending.revents == POLLIN, "DRM readiness");
    drmEventContext events = {.version = 2, .page_flip_handler = flip_received};
    require(drmHandleEvent(original.fd, &events) == 0, "DRM event");
    return 1;
}

static void submit(unsigned index)
{
    require(index < 3, "buffer index");
    int result;
    do
    {
        result = drmModePageFlip(original.fd, original.crtc, original.buffers[index].fb,
                                 DRM_MODE_PAGE_FLIP_EVENT, (void *)(uintptr_t)index);
        if (result == 0 || errno != EBUSY)
            break;
        usleep(1000);
    } while (1);
    require(result == 0, "flip admission");
    printf("T1185_SUBMIT token=%u at_ms=%lld\n", index, now_ms());
}

static void blocking_modeset(void)
{
    long long start = now_ms();
    puts("T1185_BLOCKING_MODESET_BEGIN");
    int result = drmModeSetCrtc(original.fd, original.crtc, original.buffers[0].fb, 0, 0,
                                &original.connector, 1, &original.mode);
    printf("T1185_BLOCKING_MODESET_END result=%d elapsed_ms=%lld errno=%d\n", result,
           now_ms() - start, errno);
    require(result == 0, "paused modeset result");
}

static void run_trial(int modeset)
{
    control(1, 1);
    start_scanout();
    submit(1);
    require(observe_event(2000), "active completion observation");
    require(event_count == 1 && last_token == 1, "active event identity");
    control(2, 0);
    submit(2);
    int paused_event = observe_event(12000);
    printf("T1185_PAUSED_OBSERVATION event=%d count=%u at_ms=%lld\n", paused_event, event_count,
           now_ms());
    require(!paused_event && event_count == 1, "expected deferred pause counterexample");
    if (modeset)
        blocking_modeset();
    control(3, 1);
    require(observe_event(2000), "resume original completion observation");
    require(event_count == 2 && last_token == 2, "original deferred event identity");
    require(!observe_event(100), "no duplicate completion");
    printf("T1185_COUNTEREXAMPLE_OBSERVED active=complete paused=deferred original-event=settled "
           "intervening_modeset=%d\n",
           modeset);
}

int main(int argc, char **argv)
{
    setbuf(stdout, NULL);
    require_guest();
    require(atexit(cleanup) == 0, "cleanup registration");
    require(argc <= 2, "argument extent");
    int modeset = argc == 2;
    if (modeset)
        require(strcmp(argv[1], "modeset") == 0, "diagnostic mode");
    start_helper();
    run_trial(modeset);
    return 0;
}
