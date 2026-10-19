/* T767: repeatable boundary mutations; included by the fake-device harness.
 * Real allocations and initialized owners remain valid. Fuzz values crossing
 * parsers/validators, not invented pointers or undocumented internal states. */
static uint32_t fuzz_word(const unsigned char *data, unsigned index) {
    uint32_t value;
    memcpy(&value, data + 4 * index, sizeof(value));
    return value;
}

static int fuzz_integer(const unsigned char *data, unsigned index) {
    int32_t value;
    memcpy(&value, data + 4 * index, sizeof(value));
    return value;
}

static void fuzz_text_fields(const unsigned char *data)
{
    char text[160] = {0};
    memcpy(text, data, 128);
    int capacity = conversion_capacity(text);
    assert(capacity >= 0 && capacity <= MAX_CONV_THREADS);
    int amount = capacity_value(text, (int)data[0] - 128);
    assert(amount == 1 || amount == 2 || amount == 4 || amount == 8);
    /* Invalid fd cannot authorize a sparse-idle lease, regardless of fields. */
    assert(idle_control_value(text, (int)data[1] - 32, -1, fuzz_word(data, 1)) == 0);
}

static void fuzz_mode_dimensions(const unsigned char *data) {
    frame_exchange_t frames = FRAME_EXCHANGE_INITIALIZER;
    atomic_int running = 1;
    capture_context_t capture = CAPTURE_INITIALIZER(&frames, NULL, &running);
    capture.scale = fuzz_integer(data, 2);
    if (data[12] & 1) capture.scale = 1 + data[13] % 4;
    struct evdi_mode mode = {fuzz_integer(data, 0), fuzz_integer(data, 1), 60, 32, 0x34325258};
    int valid = validate_mode_dimensions(&capture, mode);
    if (valid) {
        assert(capture.scale >= 1 && capture.scale <= 4);
        assert(mode.width >= 2 * capture.scale && mode.height >= 2 * capture.scale);
        uint64_t stride = ((uint64_t)mode.width * 4 + 63) / 64 * 64;
        assert(stride * (uint64_t)mode.height <= INT_MAX);
    } else {
        assert(!running && capture.capture_failed);
    }
    pthread_mutex_destroy(&frames.mutex);
}

static void fuzz_cursor_image(const unsigned char *data) {
    unsigned char pixels[4096], surface[16 * 80], original[sizeof(surface)];
    memset(pixels, data[31], sizeof(pixels));
    memset(surface, 0x5a, sizeof(surface));
    memcpy(original, surface, sizeof(surface));
    cursor_overlay_t cursor = {.x=fuzz_integer(data, 4), .y=fuzz_integer(data, 5)};
    cursor_image_t image = {pixels, fuzz_word(data, 3) % (sizeof(pixels) + 1),
        fuzz_word(data, 0), fuzz_word(data, 1), fuzz_word(data, 2), data[29], data[30]};
    if (data[28] & 1) {
        image.width %= 17; image.height %= 17; image.stride %= 81;
    }
    int accepted = cursor_overlay_set(&cursor, &image);
    if (accepted) {
        cursor_rect_t rect = cursor_overlay_draw(&cursor, surface, 16, 16, 80);
        assert(rect.x1 >= 0 && rect.y1 >= 0 && rect.x2 <= 16 && rect.y2 <= 16);
        cursor_overlay_restore(&cursor, surface, 80);
        assert(!memcmp(surface, original, sizeof(surface)));
    }
    cursor_overlay_destroy(&cursor);
    cursor_overlay_destroy(&cursor);
}

static void fuzz_raw_control(const unsigned char *data) {
    raw_ring_t ring = RAW_RING_INITIALIZER;
    /* No mode means HELLO only negotiates metadata; never maps large pixels. */
    int accepted = raw_control(&ring, data);
    if (accepted) assert(ring.slots >= 2 && ring.slots <= RAW_MAX_SLOTS);
    assert(!ring.memory && ring.fd == -1);
    raw_ring_close(&ring);
}

static void fuzz_raw_geometry(const unsigned char *data) {
    raw_ring_t ring = RAW_RING_INITIALIZER;
    ring.slots = fuzz_word(data, 2);
    if (data[12] & 1) ring.slots = 2 + data[13] % 7;
    uint32_t width = fuzz_word(data, 0), height = fuzz_word(data, 1);
    int valid = raw_ring_resize(&ring, width, height);
    int expected = width >= 2 && width <= 4096 && height >= 2 && height <= 4096;
    expected = expected && !((width | height) & 1) && ring.slots >= 2 && ring.slots <= 8;
    assert(valid == expected);
    assert(!ring.memory); /* No peer handshake, therefore no allocation. */
    raw_ring_close(&ring);
}

static void fuzz_damage(const unsigned char *data) {
    frame_exchange_t frames = FRAME_EXCHANGE_INITIALIZER;
    frame_exchange_resize(&frames, 32, 16);
    assert(frame_exchange_allocated(&frames));
    memset(frames.dirty_fill, 0, frames.dirty_bytes);
    int scale = fuzz_integer(data, 4);
    if (data[20] & 1) scale = 1 + data[21] % 4;
    frame_exchange_damage_rect(&frames, fuzz_integer(data, 0), fuzz_integer(data, 1),
        fuzz_integer(data, 2), fuzz_integer(data, 3), scale);
    for (int row = 0; row < frames.chroma_rows; row++) {
        if (!(frames.dirty_fill[row / 8] & (1u << (row % 8)))) continue;
        pixel_row_t span = frames.spans_fill[row];
        assert(span.begin >= 0 && span.begin < span.end && span.end <= frames.width);
        assert(!((span.begin | span.end) & 1));
    }
    frame_exchange_free(&frames);
    pthread_mutex_destroy(&frames.mutex);
}

static void fuzz_boundary_input(const unsigned char data[128]) {
    fuzz_text_fields(data);
    fuzz_mode_dimensions(data);
    fuzz_cursor_image(data);
    fuzz_raw_control(data);
    fuzz_raw_geometry(data);
    fuzz_damage(data);
}

static void fuzz_canonical_packets(void) {
    raw_ring_t sender = RAW_RING_INITIALIZER;
    assert(raw_ring_resize(&sender, 32, 16));
    sender.nonce = 17;
    unsigned char data[128] = {0};
    raw_message(&sender, data, 4, 0, 0);
    fuzz_raw_control(data);
    for (unsigned byte = 0; byte < RAW_MESSAGE_BYTES; byte++) {
        for (unsigned bit = 0; bit < 8; bit++) {
            data[byte] ^= (unsigned char)(1u << bit);
            fuzz_raw_control(data);
            data[byte] ^= (unsigned char)(1u << bit);
        }
    }
    raw_ring_close(&sender);
}

static void fuzz_edge_fields(void) {
    const int edges[] = {INT_MIN, -1, 0, 1, 2, 3, 4, 8, 16, 4095, 4096, 4097, INT_MAX};
    const int positions[] = {INT_MIN, -17, -1, 0, 1, 15, 16, 17, INT_MAX};
    for (unsigned x = 0; x < sizeof(edges) / sizeof(edges[0]); x++) {
        for (unsigned y = 0; y < sizeof(edges) / sizeof(edges[0]); y++) {
            for (unsigned p = 0; p < sizeof(positions) / sizeof(positions[0]); p++) {
                unsigned char data[128] = {0};
                int fields[] = {edges[x], edges[y], 64, 1025, positions[p], positions[p]};
                memcpy(data, fields, sizeof(fields));
                data[28] = data[29] = data[30] = 1;
                fuzz_boundary_input(data);
            }
        }
    }
}

static void test_t767_boundaries(void) {
    uint32_t seed = 767;
    unsigned char data[128];
    for (unsigned iteration = 0; iteration < 8192; iteration++) {
        for (unsigned i = 0; i < sizeof(data); i++) {
            seed = seed * 1664525u + 1013904223u;
            data[i] = (unsigned char)(seed >> 24);
        }
        fuzz_boundary_input(data);
    }
    fuzz_canonical_packets();
    fuzz_edge_fields();
}
