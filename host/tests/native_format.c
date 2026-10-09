/* T783: native channel order avoids a second registered-size framebuffer. */
static void t783_red_frame(void) {
    for (int y = 0; y < g_capture.mode_h; y++) for (int x = 0; x < g_capture.mode_w; x++) {
        unsigned char *p = g_capture.framebuffer + y * g_capture.mode_stride + x * 4;
        p[0] = 255; p[1] = 0; p[2] = 0; p[3] = 255;
    }
}

static void t783_single_buffer(void) {
    assert(t752_registered.buffer == g_capture.framebuffer && !g_capture.native_framebuffer
        && "T783: native conversion must retain only one source framebuffer");
}

static void test_t783_native_mode(void) {
    frame_exchange_init(&g_frames); g_running = 1;
    on_mode_changed((struct evdi_mode){8,8,60,32,0x34324258}, &g_capture);
    assert(g_capture.have_mode);
    t783_single_buffer();
    t783_red_frame();
    t752_grabs = 1; t752_count = 64;
    for (int i = 0; i < 64; i++) t752_rects[i] = (struct evdi_rect){0,0,1,1};
    t779_normalized_pixels = 0; grab_now(&g_capture);
    unsigned char output[96]; capture_to_nv12(&g_capture, g_capture.framebuffer, output, NULL, NULL);
    assert(output[0] == 63 && output[63] == 63 && output[64] == 102 && output[65] == 240);
    assert(t779_normalized_pixels == 0 && "T783: native path must not run the framebuffer swizzle");
    unsigned char *registered = t752_registered.buffer;
    on_mode_changed((struct evdi_mode){8,8,60,32,0x34324241}, &g_capture);
    assert(t752_registered.buffer == registered);
    on_mode_changed((struct evdi_mode){10,10,60,32,0x34324258}, &g_capture);
    assert(g_capture.mode_w == 10 && !g_capture.native_framebuffer);
    on_mode_changed((struct evdi_mode){8,8,60,32,0x34325258}, &g_capture);
    assert(!g_capture.swap_rb && !g_capture.native_framebuffer);
    free(g_capture.framebuffer); frame_exchange_free(&g_frames); pthread_cond_destroy(&g_frames.ready);
}

static void test_t783_native_cursor(void) {
    frame_exchange_init(&g_frames); g_running = 1;
    on_mode_changed((struct evdi_mode){8,8,60,32,0x34324258}, &g_capture);
    t783_single_buffer(); t783_red_frame();
    struct evdi_cursor_set event = {.enabled=1, .width=2, .height=2, .stride=12,
        .buffer_length=24, .pixel_format=0x34325241};
    event.buffer = calloc(1,24); assert(event.buffer);
    for (int y=0;y<2;y++) for (int x=0;x<2;x++) event.buffer[y*3+x] = 0x80000064;
    on_cursor_set(event, &g_capture);
    assert(*(uint32_t *)g_capture.framebuffer == 0xff64007f && "T783: blend BGRA cursor into RGBA surface");
    on_cursor_move((struct evdi_cursor_move){.x=-1,.y=-1}, &g_capture);
    assert(*(uint32_t *)g_capture.framebuffer == 0xff64007f);
    on_cursor_set((struct evdi_cursor_set){0}, &g_capture);
    assert(*(uint32_t *)g_capture.framebuffer == 0xff0000ff && "T783: restore original native byte order");
    unsigned char output[96]; capture_to_nv12(&g_capture, g_capture.framebuffer, output, NULL, NULL);
    assert(output[0] == 63 && output[64] == 102 && output[65] == 240);
    free(g_capture.framebuffer); frame_exchange_free(&g_frames); pthread_cond_destroy(&g_frames.ready);
}

static void test_t783_format_options(void) {
    assert(configure_capture_format(NULL) && !g_capture.normalize_rgba);
    assert(configure_capture_format("canonical") && g_capture.normalize_rgba);
    assert(configure_capture_format("native") && !g_capture.normalize_rgba);
    const char *invalid[] = {"", "junk", "NATIVE", "canonical-tail", "-1", " native"};
    for (unsigned i=0;i<sizeof(invalid)/sizeof(invalid[0]);i++) {
        assert(!setenv("BLENT_CAPTURE_FORMAT", invalid[i], 1));
        char *argv[] = {"helper", "--edid", "/must-not-open"};
        assert(evdi_helper_main(3,argv) == 2);
        assert(!mock_discovery_calls && !mock_add_calls);
    }
    assert(!unsetenv("BLENT_CAPTURE_FORMAT"));
}
