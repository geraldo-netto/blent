#ifndef BLENT_PIXEL_DAMAGE_H
#define BLENT_PIXEL_DAMAGE_H
#include "pixel_span.h"
#include <stdint.h>
#include <string.h>

/* CPU-owned histories. Updating metadata never grants access to leased pixels.
 * Dimensions and backing storage are validated by the buffer owner. */
/* OR one bit range without revisiting every row of overlapping rectangles. */
static inline void pixel_damage_range(unsigned char *mask, int first, int end) {
    if (first >= end) return;
    int begin_byte = first / 8, last_byte = (end - 1) / 8;
    unsigned char left = (unsigned char)(0xFFu << (first & 7));
    unsigned char right = (unsigned char)(0xFFu >> (7 - ((end - 1) & 7)));
    if (begin_byte == last_byte) { mask[begin_byte] |= left & right; return; }
    mask[begin_byte] |= left;
    memset(mask + begin_byte + 1, 0xFF, (size_t)(last_byte - begin_byte - 1));
    mask[last_byte] |= right;
}

/* Normalize in source space before rounding. Wider arithmetic also handles
 * INT_MIN/INT_MAX damage reported outside a validated framebuffer. */
static inline pixel_span_t pixel_chroma_range(int begin, int end, int scale, int limit) {
    if (end < begin) { int swap = begin; begin = end; end = swap; }
    if (begin == end) return (pixel_span_t){0};
    int64_t first = begin, last = end, divisor = 2 * scale;
    if (first < 0) first = 0;
    if (last > (int64_t)limit * divisor) last = (int64_t)limit * divisor;
    if (first >= last) return (pixel_span_t){0};
    return (pixel_span_t){(int)(first / divisor), (int)((last + divisor - 1) / divisor)};
}

static inline pixel_span_t pixel_larger_gap(pixel_span_t a, pixel_span_t b) {
    return b.end - b.begin > a.end - a.begin ? b : a;
}

static inline pixel_span_t pixel_remaining_gap(pixel_span_t gap, pixel_span_t x) {
    int left_end = x.begin < gap.end ? x.begin : gap.end;
    int right_begin = x.end > gap.begin ? x.end : gap.begin;
    pixel_span_t left = pixel_larger_gap((pixel_span_t){0}, (pixel_span_t){gap.begin, left_end});
    return pixel_larger_gap(left, (pixel_span_t){right_begin, gap.end});
}

static inline void pixel_row_merge(pixel_row_t *row, pixel_span_t x) {
    pixel_span_t gap = pixel_remaining_gap(row->gap, x);
    gap = pixel_larger_gap(gap, (pixel_span_t){row->end, x.begin});
    gap = pixel_larger_gap(gap, (pixel_span_t){x.end, row->begin});
    if (x.begin < row->begin) row->begin = x.begin;
    if (x.end > row->end) row->end = x.end;
    row->gap = gap;
}

static inline void pixel_damage_merge(unsigned char *mask, pixel_row_t *spans, pixel_span_t rows, pixel_span_t x) {
    if (!spans) return;
    for (int cy = rows.begin; cy < rows.end; cy++) {
        if (!(mask[cy / 8] & (1u << (cy % 8))))
            spans[cy] = (pixel_row_t){.begin=x.begin, .end=x.end};
        else pixel_row_merge(&spans[cy], x);
    }
}

static inline void pixel_damage_region(unsigned char *mask, pixel_row_t *spans,
                                        pixel_span_t rows, pixel_span_t x) {
    pixel_damage_merge(mask, spans, rows, x);
    pixel_damage_range(mask, rows.begin, rows.end);
}
#endif
