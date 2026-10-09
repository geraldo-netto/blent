/* T760: queued callbacks after failed retirement must leave retained storage
 * untouched and must not resurrect capture. */
static void test_t760_failed_mode(void) {
    frame_exchange_init(&g_frames);
    g_running = 1;
    allocation_countdown = 100; /* Exercise ordinary malloc fallback. */
    on_mode_changed((struct evdi_mode){8,8,60,32,0x34325258}, &g_capture);
    allocation_countdown = 0;
    on_cursor_move((struct evdi_cursor_move){0,7}, &g_capture);
    struct evdi_cursor_set event = {.enabled=1, .width=1, .height=1,
        .stride=4, .buffer_length=4, .pixel_format=0x34325241};
    event.buffer = malloc(4); assert(event.buffer); *event.buffer = 0xffffffff;
    on_cursor_set(event, &g_capture);
    unsigned char saved[512];
    assert(g_capture.fb_size == sizeof(saved));
    memcpy(saved, g_capture.framebuffer, sizeof(saved));
    unsigned char *original = g_capture.framebuffer;
    g_frames.writer_busy = 1;
    on_mode_changed((struct evdi_mode){640,480,60,32,0x34325258}, &g_capture);
    assert(g_capture.capture_failed && !g_running);
    on_cursor_move((struct evdi_cursor_move){1,7}, &g_capture);
    event.buffer = malloc(4); assert(event.buffer); *event.buffer = 0xff000000;
    on_cursor_set(event, &g_capture); /* Must still free transferred image. */
    on_update_ready(0, &g_capture);
    g_frames.writer_busy = 0;
    on_mode_changed((struct evdi_mode){16,16,60,32,0x34325258}, &g_capture);
    assert(g_capture.framebuffer == original && g_capture.mode_stride == 64);
    assert(g_capture.mode_w == 8 && g_capture.mode_h == 8 && g_capture.fb_size == 512);
    assert(!memcmp(saved, original, sizeof(saved)) && !g_capture.have_mode);
    cursor_overlay_destroy(&g_capture.cursor);
    evdi_unregister_buffer(g_capture.handle, 0);
    free(g_capture.framebuffer);
    frame_exchange_free(&g_frames);
    pthread_cond_destroy(&g_frames.ready);
    pthread_mutex_destroy(&g_frames.mutex);
}
