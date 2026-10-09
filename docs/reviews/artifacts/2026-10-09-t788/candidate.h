#ifndef BLENT_CONVERSION_BOX_H
#define BLENT_CONVERSION_BOX_H
/* Private AVX2 box reducers. Each load covers exactly two output pixels;
 * callers supply validated even spans. Sums fit before unsigned truncation. */
#include <immintrin.h>

__attribute__((target("avx2"), always_inline)) static inline __m128i box_two(const unsigned char *row, int stride) {
    const __m128i order = _mm_setr_epi8(0,4,1,5,2,6,3,7,8,12,9,13,10,14,11,15);
    const __m128i ones = _mm_set1_epi8(1);
    __m128i a = _mm_maddubs_epi16(_mm_shuffle_epi8(_mm_loadu_si128((const __m128i *)row), order), ones);
    __m128i b = _mm_maddubs_epi16(_mm_shuffle_epi8(_mm_loadu_si128((const __m128i *)(row + stride)), order), ones);
    return _mm_packus_epi16(_mm_srli_epi16(_mm_add_epi16(a, b), 2), _mm_setzero_si128());
}

__attribute__((target("avx2"), always_inline)) static inline __m128i box_four(const unsigned char *row, int stride) {
    const __m256i order = _mm256_setr_epi8(0,4,8,12,1,5,9,13,2,6,10,14,3,7,11,15,
        0,4,8,12,1,5,9,13,2,6,10,14,3,7,11,15);
    __m256i sum = _mm256_setzero_si256();
    for (int y = 0; y < 4; y++) {
        __m256i bytes = _mm256_loadu_si256((const __m256i *)(row + (size_t)y * stride));
        __m256i pairs = _mm256_maddubs_epi16(_mm256_shuffle_epi8(bytes, order), _mm256_set1_epi8(1));
        sum = _mm256_add_epi32(sum, _mm256_madd_epi16(pairs, _mm256_set1_epi16(1)));
    }
    __m256i words = _mm256_packus_epi32(_mm256_srli_epi32(sum, 4), _mm256_setzero_si256());
    __m256i bytes = _mm256_packus_epi16(words, _mm256_setzero_si256());
    return _mm_unpacklo_epi32(_mm256_castsi256_si128(bytes), _mm256_extracti128_si256(bytes, 1));
}

__attribute__((target("avx2"), always_inline)) static inline void box_row(
        unsigned char *destination, const unsigned char *source, int stride, int width, int scale) {
    for (int x = 0; x < width; x += 2) {
        const unsigned char *pixels = source + (size_t)x * scale * 4;
        __m128i pair = scale == 2 ? box_two(pixels, stride) : box_four(pixels, stride);
        _mm_storel_epi64((__m128i *)(destination + x * 4), pair);
    }
}

/* Two 128-pixel rows stay in L1 and reuse the exact unscaled color arithmetic.
 * Original output stride is retained, while the temporary source stride is fixed. */
__attribute__((target("avx2"), always_inline)) static inline void convert_box_span(
        const conv_job_t *job, int cy, pixel_span_t span, int scale, int swap_rb) {
    for (int x = span.begin; x < span.end; x += 2) {
        unsigned char pixels[16];
        const unsigned char *source = job->src + (size_t)cy * 2 * scale * job->stride + (size_t)x * scale * 4;
        __m128i top = scale == 2 ? box_two(source, job->stride) : box_four(source, job->stride);
        source += (size_t)scale * job->stride;
        __m128i bottom = scale == 2 ? box_two(source, job->stride) : box_four(source, job->stride);
        _mm_storeu_si128((__m128i *)pixels, _mm_unpacklo_epi64(top, bottom));
        conv_job_t reduced = {.src=pixels, .stride=8, .ow=job->ow,
            .ydst=job->ydst + (size_t)cy * 2 * job->ow + x,
            .uvdst=job->uvdst + (size_t)cy * job->ow + x};
        convert_native_span(&reduced, 0, (pixel_span_t){0, 2}, swap_rb);
    }
}

__attribute__((target("avx2"), always_inline)) static inline void convert_box_strip(
        const conv_job_t *job, int scale, int adaptive, int swap_rb) {
    for (int cy = job->cy0; cy < job->cy1; cy++) {
        if (!row_is_dirty(job->dirty, cy)) continue;
        pixel_row_t row = row_span(job, cy);
        int split = adaptive && pixel_row_split(row);
        for (int part = 0; part <= split; part++) convert_box_span(job, cy, row_part(row, part, split), scale, swap_rb);
    }
}
#endif
