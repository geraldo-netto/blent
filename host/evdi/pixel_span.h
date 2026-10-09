#ifndef BLENT_PIXEL_SPAN_H
#define BLENT_PIXEL_SPAN_H
/* Half-open pixel interval. Conversion spans use even output coordinates:
 * each interval contains complete 2x2 NV12 chroma blocks. No native OS types. */
typedef struct { int begin, end; } pixel_span_t;
/* Clip an unordered source interval without arithmetic on untrusted endpoints.
 * limit is a nonnegative dimension validated by the buffer owner. */
static inline pixel_span_t pixel_span_clipped(int begin, int end, int limit) {
    if (end < begin) { int swap = begin; begin = end; end = swap; }
    if (begin < 0) begin = 0;
    if (end > limit) end = limit;
    if (begin >= end) return (pixel_span_t){0};
    return (pixel_span_t){begin, end};
}
#endif
