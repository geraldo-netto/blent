/* T1021: real Rust exec/socket handoff into the production capture event loop,
 * with only EVDI device calls replaced by the permanent native harness. */
#define _GNU_SOURCE
#include <sys/types.h>
#include <sys/socket.h>
static ssize_t worker_send(int fd, const void *bytes, size_t length, int flags);
#define send worker_send
#define BLENT_C_FUZZ_DRIVER
#include "evdi_helper_test.c"
#undef send

static unsigned char *worker_framebuffer;
static int worker_paused_grabs;
static int worker_paused_requests;

static ssize_t worker_send(int fd, const void *bytes, size_t length, int flags)
{
    capture_activity_t request;
    assert(capture_activity_decode(bytes, length, &request));
    assert(g_capture.framebuffer == worker_framebuffer && g_capture.buffer_registered);
    assert(mock_disconnect_calls == 0);
    if (request.active)
    {
        assert(mock_grab_calls == worker_paused_grabs + 1);
        assert(t623_request_calls == worker_paused_requests);
    }
    else
    {
        assert(frame_exchange_quiescent(&g_frames));
        worker_paused_grabs = mock_grab_calls;
        worker_paused_requests = t623_request_calls;
    }
    return send(fd, bytes, length, flags);
}

int main(int argc, char **argv)
{
    alarm(15);
    helper_options_t options = parse_helper_options(argc, argv);
    assert(!options.invalid && options.activity_fd);
    initialize_helper_runtime();
    assert(configure_capture_channels(&options));
    mock_grab_calls = 0;
    t623_request_calls = 0;
    struct evdi_device_context *handle = malloc(sizeof(*handle));
    assert(handle);
    handle->fd = eventfd(0, EFD_NONBLOCK | EFD_CLOEXEC);
    assert(handle->fd >= 0);
    g_capture.handle = handle;
    on_mode_changed((struct evdi_mode){8, 8, 60, 32, 0x34325258}, &g_capture);
    worker_framebuffer = g_capture.framebuffer;
    assert(capture_run(&g_capture, handle) == 0);
    assert(mock_disconnect_calls == 0 && g_capture.framebuffer == worker_framebuffer);
    shutdown_capture(handle, 0);
    assert(mock_disconnect_calls == 1 && !g_capture.control);
    return 0;
}
