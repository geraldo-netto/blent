/* T764: independent black-pixel oracle with white row padding. */
#define BLENT_EXCHANGE_GENERATION
#define main benchmark_main
#include "../../scripts/benchmarks/conversion.c"
#undef main

int main(void) {
    struct lane lane = {.pool=CONV_POOL_INITIALIZER,
        .frames=FRAME_EXCHANGE_INITIALIZER, .width=4, .height=4, .scale=1};
    initialize(&lane, 0, 1);
    memset(lane.source, 255, (size_t)lane.stride * lane.height);
    for (int y = 0; y < lane.height; y++) memset(lane.source + y * lane.stride, 0, lane.width * 4);
    frame_exchange_mark_all(&lane.frames);
    convert(&lane);
    for (int i = 0; i < 16; i++) assert(lane.frames.fill[i] == 16);
    for (int i = 16; i < 24; i++) assert(lane.frames.fill[i] == 128);
    destroy(&lane);
    return 0;
}
