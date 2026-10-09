#ifndef BLENT_PIXEL_SPAN_H
#define BLENT_PIXEL_SPAN_H
/* Half-open pixel interval. Conversion spans use even output coordinates:
 * each interval contains complete 2x2 NV12 chroma blocks. No native OS types. */
typedef struct { int begin, end; } pixel_span_t;
/* Bounded row history: the enclosing interval minus one retained unchanged
 * gap. Other gaps may conservatively become dirty; changed pixels never vanish.
 * Zero-initialized gap means one interval. Coordinates are validated/even. */
typedef struct { int begin, end; pixel_span_t gap; } pixel_row_t;

/* Split only when skipping a substantial fraction of the row's work. Tiny
 * gaps and fragmented damage retain long, vectorizable conversion loops. */
static inline int pixel_row_split(pixel_row_t row) {
    int gap = row.gap.end - row.gap.begin;
    return gap >= 128 && gap >= (row.end - row.begin) / 2;
}
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
