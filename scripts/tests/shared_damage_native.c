/* T1288: independent grayscale oracle for bounded current-source replay. */
#define main benchmark_main
#include "../benchmarks/shared-damage.c"
#undef main

static unsigned reference_level(const replay_t *r, int x, int y, unsigned step)
{
    if (!r->mode)
        return 0;
    if (r->mode == 2)
        return step;
    int left = x * r->scale, top = y * r->scale;
    int right = left + r->scale, bottom = top + r->scale;
    int width = (right < 164 ? right : 164) - (left > 100 ? left : 100);
    int height = (bottom < 164 ? bottom : 164) - (top > 100 ? top : 100);
    if (width <= 0 || height <= 0)
        return 0;
    return step * width * height / (r->scale * r->scale);
}

static void verify_pixels(replay_t *r, unsigned slot, unsigned step)
{
    unsigned char *pixels = r->ring.memory + RAW_CONTROL_BYTES + slot * r->ring.slot_bytes;
    uint64_t expected = r->hash;
    for (int y = 0; y < r->ring.height; y++)
    {
        for (int x = 0; x < r->ring.width; x++)
        {
            unsigned level = reference_level(r, x, y, step);
            unsigned char value = 16 + (220 * level + 128) / 256;
            assert(pixels[y * r->ring.width + x] == value);
            expected = (expected ^ value) * UINT64_C(1099511628211);
        }
    }
    size_t size = (size_t)r->ring.width * r->ring.height;
    for (size_t i = size; i < size * 3 / 2; i++)
    {
        assert(pixels[i] == 128);
        expected = (expected ^ 128) * UINT64_C(1099511628211);
    }
    hash_slot(r, slot);
    assert(r->hash == expected);
}

static void replay_case(int scale, int mode)
{
    replay_t r = {.scale = scale, .mode = mode};
    initialize(&r);
    for (unsigned step = 0; step < 12; step++)
    {
        unsigned slot = step % RAW_SLOTS;
        atomic_store((_Atomic uint32_t *)(r.ring.memory + slot * RAW_SLOT_CONTROL_BYTES), RAW_FREE);
        mutate(&r, step * 19);
        r.capture.raw_sent_us = 0;
        assert(publish_shared_capture(&r.capture));
        verify_pixels(&r, slot, step * 19);
        drain(&r);
    }
    raw_ring_close(&r.ring);
    close(r.peer);
    conv_pool_destroy(&r.pool);
    free(r.capture.framebuffer);
    assert(pthread_cond_destroy(&r.frames.ready) == 0);
    assert(pthread_mutex_destroy(&r.frames.mutex) == 0);
}

int main(void)
{
    for (int scale = 1; scale <= 4; scale++)
        for (int mode = 0; mode <= 2; mode++)
            replay_case(scale, mode);
    return 0;
}
