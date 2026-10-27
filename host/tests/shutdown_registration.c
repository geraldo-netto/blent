/* T1284: registration owns a borrowed native buffer until explicit removal. */
static void t1284_shutdown_case(int mode)
{
    initialize_helper_runtime();
    t1284_tracking = 1;
    g_running = 1;
    evdi_handle handle = calloc(1, sizeof(*handle));
    assert(handle);
    handle->fd = eventfd(0, EFD_CLOEXEC);
    assert(handle->fd >= 0);
    g_capture.handle = handle;
    g_capture.normalize_rgba = 1;
    if (mode == 3)
        allocation_countdown = 1;
    if (mode)
    {
        unsigned format = mode == 2 ? 0x34324241 : 0x34325258;
        on_mode_changed((struct evdi_mode){8, 8, 60, 32, format}, &g_capture);
    }
    allocation_countdown = 0;
    assert(!!t1284_registration == (mode == 1 || mode == 2));
    shutdown_capture(handle, 0);
    assert(!t1284_registration && !g_capture.buffer_registered);
    assert(mock_disconnect_calls == 1 && !g_capture.handle);
    assert(pthread_cond_destroy(&g_frames.ready) == 0);
    assert(pthread_mutex_destroy(&g_frames.mutex) == 0);
}

static void test_t1284_shutdown_registration(void)
{
    for (int mode = 0; mode < 4; mode++)
    {
        pid_t child = fork();
        assert(child >= 0);
        if (!child)
        {
            t1284_shutdown_case(mode);
            exit(0);
        }
        int status;
        assert(waitpid(child, &status, 0) == child);
        assert(WIFEXITED(status) && WEXITSTATUS(status) == 0);
    }
}
