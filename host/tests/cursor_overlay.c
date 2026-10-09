/* T753: independent cursor contract, bounded malformed inputs and adapter life. */
static void t789_take_variants(void) {
    cursor_overlay_t cursor = {0};
    for (unsigned variant = 0; variant < 8; variant++) {
        size_t size = variant == 2 ? 24 : 20;
        unsigned char *pixels = malloc(size); assert(pixels); memset(pixels, 127, size);
        cursor_image_t image = {pixels, size, 2, 2, variant == 2 ? 12 : 8,
            variant != 0, variant == 1};
        if (variant >= 4) image.length = variant - 4; /* Reject truncated ownership transfers. */
        int valid = variant < 4;
        unsigned char *previous = cursor.storage;
        assert(cursor_overlay_take(&cursor, &image) == valid && !image.pixels);
        if (valid) assert(cursor.storage != pixels); /* Normalized or excess allocation detached. */
        else assert(cursor.storage == previous);
    }
    cursor_overlay_destroy(&cursor);
}

static void t789_take_failures(void) {
    cursor_overlay_t cursor = {0};
    unsigned char previous[16] = {0};
    cursor_image_t borrowed = {previous, sizeof(previous), 2, 2, 8, 1, 0};
    assert(cursor_overlay_set(&cursor, &borrowed));
    unsigned char *retained = cursor.storage;
    for (int attempt = 0; attempt < 2; attempt++) {
        unsigned char *pixels = malloc(16); assert(pixels); memset(pixels, 127, 16);
        cursor_image_t image = {pixels, 16, 2, 2, 8, 1, attempt != 0};
        allocation_countdown = 1;
        assert(!cursor_overlay_take(&cursor, &image));
        assert(!image.pixels && cursor.storage == retained);
        allocation_countdown = 0;
    }
    cursor_overlay_destroy(&cursor);
}

static void t789_adoption_boundary(void) {
    cursor_overlay_t cursor = {0};
    const unsigned widths[] = {128, 129, 1024};
    for (unsigned i = 0; i < 3; i++) {
        unsigned n = widths[i]; size_t bytes = (size_t)n * n * 4;
        unsigned char *pixels = malloc(bytes); assert(pixels); memset(pixels, 127, bytes);
        cursor_image_t image = {pixels, bytes, n, n, n*4, 1, 0};
        assert(cursor_overlay_take(&cursor, &image));
        assert((cursor.storage == pixels) == (n == 128));
        assert(cursor.separate_background == (n == 128));
    }
    cursor_overlay_destroy(&cursor);
}

static void test_t789_owned_cursor(void) {
    t789_take_variants();
    t789_take_failures();
    t789_adoption_boundary();
    struct evdi_cursor_set event = {.width=2, .height=2, .stride=8, .buffer_length=16,
        .pixel_format=0x34325241, .enabled=1};
    event.buffer = malloc(16); assert(event.buffer);
    memset(event.buffer, 127, 16);
    on_cursor_set(event, &g_capture);
    assert(g_capture.cursor.storage == (unsigned char *)event.buffer &&
        "T789: packed native cursor must retain the transferred allocation");
    unsigned char background[16], original[16];
    memset(background, 31, sizeof(background)); memcpy(original, background, sizeof(original));
    cursor_overlay_draw(&g_capture.cursor, background, 2, 2, 8);
    cursor_overlay_restore(&g_capture.cursor, background, 8);
    assert(!memcmp(background, original, sizeof(background)));
    on_cursor_set((struct evdi_cursor_set){0}, &g_capture);
    assert(!g_capture.cursor.storage);
}

static void t753_invalid_images(void) {
    cursor_overlay_t cursor = {0};
    unsigned char bytes[64] = {0};
    cursor_image_t image = {bytes, sizeof(bytes), 2, 2, 8, 1, 0};
    assert(cursor_overlay_set(&cursor, &image));
    unsigned char *owned = cursor.storage;
    for (unsigned i = 0; i < 32; i++) {
        cursor_image_t bad = image;
        bad.length = i;
        int ok = cursor_overlay_set(&cursor, &bad);
        assert(ok == (i >= 16));
        if (ok) owned = cursor.storage;
        assert(cursor.storage == owned);
    }
    const unsigned dimensions[] = {0, 1048577, UINT_MAX};
    for (unsigned i = 0; i < 3; i++) {
        cursor_image_t bad = image; bad.width = dimensions[i];
        assert(!cursor_overlay_set(&cursor, &bad));
        bad = image; bad.height = dimensions[i];
        assert(!cursor_overlay_set(&cursor, &bad));
    }
    image.stride = 7; assert(!cursor_overlay_set(&cursor, &image));
    image.stride = 8; image.pixels = NULL; assert(!cursor_overlay_set(&cursor, &image));
    image.pixels = bytes; allocation_countdown = 1;
    assert(!cursor_overlay_set(&cursor, &image));
    allocation_countdown = 0;
    cursor_overlay_destroy(&cursor);
}

static void t753_clip_positions(void) {
    cursor_overlay_t cursor = {0};
    unsigned char image[16]; memset(image, 255, sizeof(image));
    cursor_image_t picture = {image, sizeof(image), 2, 2, 8, 1, 0};
    assert(cursor_overlay_set(&cursor, &picture));
    const int positions[] = {INT_MIN, -3, -1, 0, 1, 3, 4, INT_MAX};
    for (unsigned i = 0; i < 8; i++) for (unsigned j = 0; j < 8; j++) {
        unsigned char surface[16 + 4*24 + 16], original[sizeof(surface)];
        memset(surface, 0x35, sizeof(surface)); memcpy(original, surface, sizeof(surface));
        cursor.x = positions[i]; cursor.y = positions[j];
        cursor_rect_t rect = cursor_overlay_draw(&cursor, surface + 16, 4, 4, 24);
        assert(rect.x1 >= 0 && rect.y1 >= 0 && rect.x2 <= 4 && rect.y2 <= 4);
        cursor_overlay_restore(&cursor, surface + 16, 24);
        assert(!memcmp(surface, original, sizeof(surface)) && "T753: clip/restore must preserve padding and guards");
    }
    assert(cursor_overlay_restore(&cursor, NULL, 0).x2 == 0);
    assert(cursor_overlay_draw(&cursor, NULL, 4, 4, 24).x2 == 0);
    cursor_overlay_destroy(&cursor);
    assert(cursor_overlay_draw(&cursor, image, 2, 2, 8).x2 == 0);
    assert(cursor_overlay_restore(&cursor, image, 8).x2 == 0);
}

static void t753_alpha_and_format(void) {
    unsigned char source[] = {32, 48, 64, 128, 0, 0, 0, 0, 255, 255, 255, 1};
    unsigned char pixels[] = {100, 100, 100, 255, 90, 91, 92, 255, 40, 40, 40, 255};
    unsigned char original[sizeof(pixels)]; memcpy(original, pixels, sizeof(pixels));
    cursor_image_t image = {source, sizeof(source), 3, 1, 12, 1, 0};
    cursor_overlay_t cursor = {0};
    assert(cursor_overlay_set(&cursor, &image));
    cursor_overlay_draw(&cursor, pixels, 3, 1, 12);
    assert(pixels[0] == 82 && pixels[1] == 98 && pixels[2] == 114);
    assert(!memcmp(pixels + 4, original + 4, 4) && pixels[8] == 255);
    cursor_overlay_restore(&cursor, pixels, 12);
    image.alpha = 0; image.swap_rb = 1;
    assert(cursor_overlay_set(&cursor, &image));
    cursor_overlay_draw(&cursor, pixels, 3, 1, 12);
    assert(pixels[0] == 64 && pixels[1] == 48 && pixels[2] == 32 && pixels[3] == 255);
    cursor_overlay_restore(&cursor, pixels, 12);
    assert(!memcmp(pixels, original, sizeof(pixels)));
    cursor_overlay_destroy(&cursor);
}

static void t753_event_lifecycle(void) {
    frame_exchange_init(&g_frames);
    t497_capture_events = 1; g_running = 1;
    const uint32_t formats[] = {0x34325241, 0x34325258, 0x34324241, 0x34324258};
    for (unsigned i=0;i<4;i++) {
        struct evdi_cursor_set event = {.width=1,.height=1,.stride=4,.buffer_length=4,
            .pixel_format=formats[i],.enabled=1,.hot_x=INT_MIN,.hot_y=INT_MAX};
        event.buffer=malloc(4); assert(event.buffer); *event.buffer=0xffffffff;
        on_cursor_set(event,&g_capture); /* image may precede mode; top-left ignores hotspot */
    }
    on_mode_changed((struct evdi_mode){8,8,60,32,0x34325258},&g_capture);
    assert(*(uint32_t*)g_capture.framebuffer==0xffffffff);
    on_mode_changed((struct evdi_mode){10,10,60,32,0x34325258},&g_capture);
    assert(*(uint32_t*)g_capture.framebuffer==0xffffffff);
    mock_grab_calls=0; t497_capture_events=0;
    grab_now(&g_capture); /* unchanged underlying desktop retains overlay */
    assert(*(uint32_t*)g_capture.framebuffer==0xffffffff);
    on_cursor_set((struct evdi_cursor_set){0},&g_capture);
    assert(*(uint32_t*)g_capture.framebuffer==0x18181818);
    struct evdi_cursor_set invalid={.enabled=1,.pixel_format=UINT_MAX,.buffer=malloc(4)};
    on_cursor_set(invalid,&g_capture);
    assert(!g_running && g_capture.capture_failed);
    cursor_overlay_destroy(&g_capture.cursor);
    free(g_capture.framebuffer);g_capture.framebuffer=NULL;
    frame_exchange_free(&g_frames);conv_pool_destroy(&g_conversion);pthread_cond_destroy(&g_frames.ready);
}

static void test_t753_bounds(void) {
    t753_invalid_images();
    t753_clip_positions();
    t753_alpha_and_format();
    t753_event_lifecycle();
}

static void test_t753_shared(void) {
    raw_ring_t ring = RAW_RING_INITIALIZER;
    int pair[2]; assert(socketpair(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK, 0, pair) == 0);
    assert(raw_ring_init(&ring, pair[0]));
    frame_exchange_init(&g_frames); g_capture.raw_ring = &ring; g_running = 1;
    on_mode_changed((struct evdi_mode){16,16,60,32,0x34325258}, &g_capture);
    raw_ring_t hello = ring; hello.nonce = 8; t418_control(&hello,pair[1],4);
    mock_monotonic_ms=1000; assert(service_shared_capture(&g_capture));
    unsigned char baseline = ring.memory[RAW_CONTROL_BYTES];
    unsigned char leased[384]; memcpy(leased,ring.memory+RAW_CONTROL_BYTES,sizeof(leased));
    struct evdi_cursor_set event={.width=2,.height=2,.stride=8,.buffer_length=16,.enabled=1,.pixel_format=0x34325241};
    event.buffer=malloc(16);assert(event.buffer);memset(event.buffer,255,16);
    on_cursor_set(event,&g_capture);
    assert(g_capture.raw_pending && !memcmp(leased,ring.memory+RAW_CONTROL_BYTES,sizeof(leased)));
    mock_monotonic_ms+=17;assert(publish_shared_capture(&g_capture));
    assert(ring.memory[RAW_CONTROL_BYTES+ring.slot_bytes]==235);
    on_cursor_set((struct evdi_cursor_set){0},&g_capture);
    mock_monotonic_ms+=17;assert(publish_shared_capture(&g_capture));
    assert(ring.memory[RAW_CONTROL_BYTES+2*ring.slot_bytes]==baseline);
    assert(!memcmp(leased,ring.memory+RAW_CONTROL_BYTES,sizeof(leased)) && "T753: cursor update cannot mutate leased slots");
    t418_drain(pair[1]);close(pair[1]);raw_ring_close(&ring);
    free(g_capture.framebuffer);g_capture.framebuffer=NULL;g_capture.raw_ring=NULL;
    frame_exchange_free(&g_frames);conv_pool_destroy(&g_conversion);pthread_cond_destroy(&g_frames.ready);
}
