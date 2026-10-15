/* T1021: independent control vectors and retained native capture ownership. */
static void test_t1021_wire(void)
{
    unsigned char bytes[32] = {0, 0, 0, 0, 0, 0, 0, 1, 1};
    capture_activity_t request;
    for (size_t length = 0; length <= sizeof(bytes); length++)
    {
        assert(capture_activity_decode(bytes, length, &request) == (length == 9));
    }
    for (int flag = 0; flag <= 255; flag++)
    {
        bytes[8] = flag;
        assert(capture_activity_decode(bytes, 9, &request) == (flag <= 1));
    }
    bytes[8] = 1;
    assert(capture_activity_decode(bytes, 9, &request));
    assert(request.sequence == 1 && request.active == 1);
    assert(capture_activity_next(request, 0));
    assert(!capture_activity_next(request, 1));
    assert(!capture_activity_next(request, UINT64_MAX));
    memset(bytes, 255, 8);
    assert(capture_activity_decode(bytes, 9, &request));
    assert(request.sequence == UINT64_MAX && capture_activity_next(request, UINT64_MAX - 1));
    memset(bytes, 0, 8);
    assert(!capture_activity_decode(bytes, 9, &request));
}

static int t1021_pair(capture_control_t *control)
{
    int pair[2];
    assert(socketpair(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK | SOCK_CLOEXEC, 0, pair) == 0);
    assert(capture_control_init(control, pair[0]));
    return pair[1];
}

static void t1021_send(int peer, uint64_t sequence, int active)
{
    unsigned char bytes[9];
    capture_activity_encode((capture_activity_t){sequence, active}, bytes);
    assert(send(peer, bytes, sizeof(bytes), MSG_NOSIGNAL) == sizeof(bytes));
}

static void t1021_ack(int peer, uint64_t sequence, int active)
{
    unsigned char bytes[10];
    ssize_t count = recv(peer, bytes, sizeof(bytes), MSG_DONTWAIT);
    capture_activity_t request;
    assert(count == 9 && capture_activity_decode(bytes, count, &request));
    assert(request.sequence == sequence && request.active == active);
}

static void test_t1021_control(void)
{
    capture_control_t control;
    assert(!capture_control_init(&control, -1));
    int unconnected = socket(AF_UNIX, SOCK_SEQPACKET, 0);
    assert(unconnected >= 0 && !capture_control_init(&control, unconnected));
    close(unconnected);
    int peer = t1021_pair(&control);
    assert(capture_control_poll(&control) == 0);
    t1021_send(peer, 1, 1);
    assert(capture_control_poll(&control) == 1);
    assert(control.current.sequence == 1 && control.current.active);
    assert(capture_control_poll(&control) == 0);
    assert(capture_control_ack(&control) == 1);
    t1021_ack(peer, 1, 1);
    t1021_send(peer, 1, 0);
    assert(capture_control_poll(&control) == -1);
    assert(control.current.active && control.current.sequence == 1);
    close(peer);
    assert(capture_control_poll(&control) == -1);
    close(control.fd);
}

static void t1021_poll(capture_control_t *control)
{
    struct evdi_event_context events = {.user_data = &g_capture};
    struct pollfd fds[3] = {{.fd = -1}, {.fd = -1}, {.fd = control->fd, .events = POLLIN}};
    assert(poll_capture_events(EVDI_INVALID_HANDLE, &events, fds, 0) == 1);
}

static void t1021_mode(capture_control_t *control)
{
    frame_exchange_init(&g_frames);
    g_running = 1;
    g_capture.control = control;
    mock_grab_calls = 0;
    t623_request_calls = 0;
    g_conversion.last_jobs = 0;
    on_mode_changed((struct evdi_mode){8, 8, 60, 32, 0x34325258}, &g_capture);
    assert(g_capture.buffer_registered && g_capture.have_mode);
    assert(g_conversion.last_jobs == 0);
}

static void t1021_idle(void)
{
    int grabs = mock_grab_calls;
    int requests = t623_request_calls;
    int jobs = g_conversion.last_jobs;
    g_capture.update_pending = 1;
    on_update_ready(0, &g_capture);
    grab_now(&g_capture);
    publish_frame(&g_capture);
    long long fallback = 0, stats = 0, base = 0;
    assert(capture_tick(&g_capture, EVDI_INVALID_HANDLE, 17, &fallback, &stats, &base));
    assert(capture_poll_timeout(&g_capture, 17) == 100);
    assert(shared_poll_timeout(&g_capture, 100) == 100);
    assert(mock_grab_calls == grabs && t623_request_calls == requests);
    assert(g_conversion.last_jobs == jobs);
}

static void test_t1021_pause(void)
{
    alarm(10);
    capture_control_t control;
    int peer = t1021_pair(&control);
    t1021_mode(&control);
    unsigned char *original = g_capture.framebuffer;
    t1021_idle();
    t1021_send(peer, 1, 1);
    t1021_poll(&control);
    t1021_ack(peer, 1, 1);
    assert(mock_grab_calls == 1 && t623_request_calls == 0);
    publish_frame(&g_capture);
    frame_cursor_t cursor = {0};
    frame_lease_t lease;
    assert(frame_exchange_claim(&g_frames, &cursor, &g_running, NULL, &lease) == 1);
    t1021_send(peer, 2, 0);
    t1021_poll(&control);
    assert(control.acknowledged == 1 && !frame_exchange_quiescent(&g_frames));
    unsigned char bytes[9];
    assert(recv(peer, bytes, sizeof(bytes), MSG_DONTWAIT) < 0 && errno == EAGAIN);
    t1021_idle();
    assert(g_capture.framebuffer == original && g_capture.buffer_registered);
    assert(!mock_disconnect_calls && g_running);
    frame_exchange_release(&g_frames);
    t1021_poll(&control);
    t1021_ack(peer, 2, 0);
    assert(frame_exchange_quiescent(&g_frames));
    t1021_send(peer, 3, 1);
    t1021_poll(&control);
    t1021_ack(peer, 3, 1);
    assert(mock_grab_calls == 2 && t623_request_calls == 0);
    assert(g_capture.framebuffer == original && !mock_disconnect_calls);
    close(peer);
    close(control.fd);
    t497_free_capture();
}

static void test_t1021_raw(void)
{
    capture_control_t control;
    int peer = t1021_pair(&control);
    raw_ring_t ring = RAW_RING_INITIALIZER;
    int raw[2];
    assert(socketpair(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK, 0, raw) == 0);
    assert(raw_ring_init(&ring, raw[0]));
    g_capture.raw_ring = &ring;
    t1021_mode(&control);
    raw_ring_t hello = ring;
    hello.nonce = 1021;
    t418_control(&hello, raw[1], 4);
    assert(service_shared_capture(&g_capture));
    t418_drain(raw[1]);
    t1021_idle();
    assert(publish_shared_capture(&g_capture));
    assert(ring.sequence == 0 && !g_frames.latest_valid);
    t1021_send(peer, 1, 1);
    t1021_poll(&control);
    t1021_ack(peer, 1, 1);
    assert(publish_shared_capture(&g_capture));
    assert(ring.sequence == 1);
    t1021_send(peer, 2, 0);
    t1021_poll(&control);
    t1021_ack(peer, 2, 0);
    t1021_idle();
    assert(publish_shared_capture(&g_capture));
    assert(ring.sequence == 1 && !mock_disconnect_calls);
    t418_drain(raw[1]);
    raw_ring_close(&ring);
    close(raw[1]);
    close(peer);
    close(control.fd);
    t497_free_capture();
}

static void t1021_invalid_length(size_t length)
{
    capture_control_t control;
    int peer = t1021_pair(&control);
    unsigned char bytes[32] = {0};
    assert(send(peer, bytes, length, MSG_NOSIGNAL) == (ssize_t)length);
    assert(capture_control_poll(&control) == -1);
    assert(control.current.sequence == 0);
    close(peer);
    close(control.fd);
}

static void t1021_blocked_ack(void)
{
    capture_control_t control;
    int peer = t1021_pair(&control);
    t1021_send(peer, 1, 0);
    assert(capture_control_poll(&control) == 1);
    unsigned char bytes[9] = {0};
    while (send(control.fd, bytes, sizeof(bytes), MSG_DONTWAIT | MSG_NOSIGNAL) > 0)
    {
    }
    assert(capture_control_ack(&control) == 0 && control.acknowledged == 0);
    while (recv(peer, bytes, sizeof(bytes), MSG_DONTWAIT) > 0)
    {
    }
    assert(capture_control_ack(&control) == 1);
    t1021_ack(peer, 1, 0);
    assert(capture_control_ack(&control) == 1);
    t1021_send(peer, 2, 1);
    assert(capture_control_poll(&control) == 1);
    close(peer);
    assert(capture_control_ack(&control) == -1 && control.acknowledged == 1);
    close(control.fd);
    assert(capture_control_ack(&control) == -1);
}

static void test_t1021_boundaries(void)
{
    for (size_t length = 0; length <= 32; length++)
        t1021_invalid_length(length);
    t1021_blocked_ack();
    assert(configure_capture_control(NULL));
    helper_options_t bad_raw = {.socket_fd = "-1"};
    assert(!configure_capture_channels(&bad_raw));
    const char *invalid[] = {"", "junk", "2", "-1", "2147483648", "3tail"};
    for (size_t i = 0; i < sizeof(invalid) / sizeof(invalid[0]); i++)
        assert(!configure_capture_control(invalid[i]));
    char *argv[] = {"helper", "--edid", "/unused", "--capture-activity-fd", "-1"};
    assert(evdi_helper_main(5, argv) == 1);
    assert(!mock_open_calls && !mock_add_calls);
    capture_control_t control;
    int peer = t1021_pair(&control);
    char value[32];
    snprintf(value, sizeof(value), "%d", control.fd);
    assert(configure_capture_control(value));
    assert(g_capture.control == &g_control && g_control.fd == control.fd);
    close(peer);
    close(control.fd);
    g_capture.control = NULL;
}

static int t1021_aliases(int fd)
{
    struct stat expected;
    assert(fstat(fd, &expected) == 0);
    DIR *directory = opendir("/proc/self/fd");
    assert(directory);
    int count = 0;
    struct dirent *entry;
    while ((entry = readdir(directory)))
    {
        if (entry->d_name[0] == '.')
            continue;
        struct stat actual;
        if (fstat(atoi(entry->d_name), &actual) == 0 && actual.st_dev == expected.st_dev &&
            actual.st_ino == expected.st_ino)
            count++;
    }
    closedir(directory);
    return count;
}

static void t1021_send_right(int peer, int fd)
{
    unsigned char bytes[9] = {0, 0, 0, 0, 0, 0, 0, 1, 0};
    struct iovec vector = {bytes, sizeof(bytes)};

    union
    {
        struct cmsghdr aligned;
        char bytes[CMSG_SPACE(sizeof(int))];
    } ancillary = {0};
    struct msghdr message = {.msg_iov = &vector,
                             .msg_iovlen = 1,
                             .msg_control = ancillary.bytes,
                             .msg_controllen = sizeof(ancillary)};
    struct cmsghdr *header = CMSG_FIRSTHDR(&message);
    header->cmsg_level = SOL_SOCKET;
    header->cmsg_type = SCM_RIGHTS;
    header->cmsg_len = CMSG_LEN(sizeof(int));
    memcpy(CMSG_DATA(header), &fd, sizeof(fd));
    assert(sendmsg(peer, &message, MSG_NOSIGNAL) == sizeof(bytes));
}

static void test_t1021_rights(void)
{
    capture_control_t control;
    int peer = t1021_pair(&control);
    int pipefd[2];
    assert(pipe(pipefd) == 0);
    int before = t1021_aliases(pipefd[0]);
    for (int i = 0; i < 32; i++)
    {
        t1021_send_right(peer, pipefd[0]);
        assert(capture_control_poll(&control) == -1);
        assert(control.current.sequence == 0 && t1021_aliases(pipefd[0]) == before);
    }
    close(peer);
    close(control.fd);
    close(pipefd[0]);
    close(pipefd[1]);
}
