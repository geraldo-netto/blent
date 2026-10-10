#include "conversion_private.h"
#include <stdio.h>
#include <string.h>

#if (defined(__x86_64__) || defined(__i386__)) && !defined(BLENT_NO_AVX2)
#define BLENT_AVX2 1
#endif

void conv_copy_rgba_to_bgra(unsigned char *destination, const unsigned char *source, size_t pixels) {
    for (size_t x = 0; x < pixels; x++, destination += 4, source += 4) {
        destination[0] = source[2]; destination[1] = source[1];
        destination[2] = source[0]; destination[3] = source[3];
    }
}

static inline int row_is_dirty(const unsigned char *mask, int cy) {
    return mask == NULL || (mask[cy >> 3] & (1u << (cy & 7)));
}

static inline pixel_row_t row_span(const conv_job_t *job, int cy) {
    return job->spans ? job->spans[cy] : (pixel_row_t){.begin=0, .end=job->ow};
}

static inline unsigned char clamp_byte(int value) {
    if (value < 0) return 0;
    if (value > 255) return 255;
    return (unsigned char)value;
}

/* BT.709 limited-range chroma from the sum of four luma samples. */
static inline void write_chroma(unsigned char *uv, int sb, int sg, int sr) {
    int ab = sb >> 2, ag = sg >> 2, ar = sr >> 2;
    uv[0] = clamp_byte(((-26 * ar - 86 * ag + 112 * ab + 128) >> 8) + 128);
    uv[1] = clamp_byte(((112 * ar - 102 * ag - 10 * ab + 128) >> 8) + 128);
}

/* Downscaling variant: every output pixel is the mean of a scale x scale
   source block, and each chroma sample the mean of the 2*scale square it
   covers. Box averaging rather than point sampling — dropping pixels would
   alias hard on desktop content, where single-pixel lines are everywhere. */
static inline void convert_scaled_span(const conv_job_t *j, int cy, int n, pixel_span_t span, int swap_rb) {
    const int stride = j->stride, ow = j->ow;
    const int inv = n * n, bi = swap_rb ? 2 : 0, ri = swap_rb ? 0 : 2;
    unsigned char *yo0 = j->ydst + (size_t)(cy * 2) * ow;
    unsigned char *yo1 = yo0 + ow;
    unsigned char *uv  = j->uvdst + (size_t)cy * ow;
    for (int ox = span.begin; ox < span.end; ox += 2) {
        int csb = 0, csg = 0, csr = 0;      /* chroma: whole 2n x 2n block */
        for (int q = 0; q < 4; q++) {       /* four output luma pixels */
            int oxx = ox + (q & 1);
            int oyy = cy * 2 + (q >> 1);
            int sb = 0, sg = 0, sr = 0;
            for (int dy = 0; dy < n; dy++) {
                const unsigned char *row =
                    j->src + (size_t)(oyy * n + dy) * stride + (size_t)(oxx * n) * 4;
                for (int dx = 0; dx < n; dx++) {
                    sb += row[dx * 4 + bi];
                    sg += row[dx * 4 + 1];
                    sr += row[dx * 4 + ri];
                }
            }
            int b = sb / inv, g = sg / inv, r = sr / inv;
            unsigned char yv =
                (unsigned char)(((47 * r + 157 * g + 16 * b + 128) >> 8) + 16);
            if (q < 2) yo0[oxx] = yv; else yo1[oxx] = yv;
            csb += b; csg += g; csr += r;
        }
        write_chroma(uv + ox, csb, csg, csr);
    }
}

static inline pixel_span_t row_part(pixel_row_t row, int part, int split) {
    if (!split) return (pixel_span_t){row.begin, row.end};
    if (part) return (pixel_span_t){row.gap.end, row.end};
    return (pixel_span_t){row.begin, row.gap.begin};
}

static inline void convert_strip_scaled(const conv_job_t *j, const int n, int adaptive, int swap_rb) {
    for (int cy = j->cy0; cy < j->cy1; cy++) {
        if (!row_is_dirty(j->dirty, cy)) continue;
        pixel_row_t row = row_span(j, cy);
        int split = adaptive && pixel_row_split(row);
        for (int part = 0; part <= split; part++)
            convert_scaled_span(j, cy, n, row_part(row, part, split), swap_rb);
    }
}

static inline __attribute__((always_inline)) void convert_native_span(const conv_job_t *j, int cy, pixel_span_t span, int swap_rb) {
    const int w = j->ow, stride = j->stride, bi = swap_rb ? 2 : 0, ri = swap_rb ? 0 : 2;
    int y0 = cy * 2, y1 = y0 + 1;
    const unsigned char *row0 = j->src + (size_t)y0 * stride;
    const unsigned char *row1 = j->src + (size_t)y1 * stride;
    unsigned char *yo0 = j->ydst + (size_t)y0 * w;
    unsigned char *yo1 = j->ydst + (size_t)y1 * w;
    unsigned char *uv  = j->uvdst + (size_t)cy * w;
    for (int x = span.begin; x < span.end; x += 2) {
        const unsigned char *p;
        int b, g, r, sb, sg, sr;
        p = row0 + (size_t)x * 4;       b = p[bi]; g = p[1]; r = p[ri];
        yo0[x]   = (unsigned char)(((47*r + 157*g + 16*b + 128) >> 8) + 16);
        sb = b; sg = g; sr = r;
        p = row0 + (size_t)(x+1) * 4;   b = p[bi]; g = p[1]; r = p[ri];
        yo0[x+1] = (unsigned char)(((47*r + 157*g + 16*b + 128) >> 8) + 16);
        sb += b; sg += g; sr += r;
        p = row1 + (size_t)x * 4;       b = p[bi]; g = p[1]; r = p[ri];
        yo1[x]   = (unsigned char)(((47*r + 157*g + 16*b + 128) >> 8) + 16);
        sb += b; sg += g; sr += r;
        p = row1 + (size_t)(x+1) * 4;   b = p[bi]; g = p[1]; r = p[ri];
        yo1[x+1] = (unsigned char)(((47*r + 157*g + 16*b + 128) >> 8) + 16);
        sb += b; sg += g; sr += r;
        write_chroma(uv + x, sb, sg, sr);
    }
}

static inline __attribute__((always_inline)) void convert_strip(const conv_job_t *j, int adaptive, int swap_rb) {
    for (int cy = j->cy0; cy < j->cy1; cy++) {
        if (!row_is_dirty(j->dirty, cy)) continue;
        pixel_row_t row = row_span(j, cy);
        int split = adaptive && pixel_row_split(row);
        for (int part = 0; part <= split; part++)
            convert_native_span(j, cy, row_part(row, part, split), swap_rb);
    }
}

/* Constant scales let the compiler simplify box sums/divisions without
 * ISA-specific code or changes to the two-stage integer rounding. */
static inline __attribute__((always_inline)) void convert_ordered_job(const conv_job_t *job, int adaptive, int swap_rb) {
    switch (job->scale) {
        case 1: convert_strip(job, adaptive, swap_rb); break;
        case 2: convert_strip_scaled(job, 2, adaptive, swap_rb); break;
        case 3: convert_strip_scaled(job, 3, adaptive, swap_rb); break;
        case 4: convert_strip_scaled(job, 4, adaptive, swap_rb); break;
        default: convert_strip_scaled(job, job->scale, adaptive, swap_rb); break;
    }
}

static void convert_job(const conv_job_t *job) { convert_ordered_job(job, 0, 0); }
static void convert_adaptive_job(const conv_job_t *job) { convert_ordered_job(job, 1, 0); }
static void convert_rgba_job(const conv_job_t *job) { convert_ordered_job(job, 0, 1); }
static void convert_rgba_adaptive_job(const conv_job_t *job) { convert_ordered_job(job, 1, 1); }

#ifdef BLENT_AVX2
#include "conversion_box.h"
__attribute__((target("avx2"), always_inline)) static inline void convert_avx2_ordered(
        const conv_job_t *job, int adaptive, int swap_rb) {
    if (job->scale == 1) convert_strip(job, adaptive, swap_rb);
    else if (job->scale == 2) convert_box_strip(job, 2, adaptive, swap_rb);
    else convert_box_strip(job, 4, adaptive, swap_rb);
}
__attribute__((target("avx2"))) static void convert_avx2_job(const conv_job_t *job) { convert_avx2_ordered(job, 0, 0); }
__attribute__((target("avx2"))) static void convert_avx2_adaptive_job(const conv_job_t *job) { convert_avx2_ordered(job, 1, 0); }
__attribute__((target("avx2"))) static void convert_avx2_rgba_job(const conv_job_t *job) { convert_avx2_ordered(job, 0, 1); }
__attribute__((target("avx2"))) static void convert_avx2_rgba_adaptive_job(const conv_job_t *job) { convert_avx2_ordered(job, 1, 1); }
#endif

int conv_simd_available(conv_simd_t policy) {
    if (policy == CONV_SIMD_AUTO || policy == CONV_SIMD_PORTABLE) return 1;
#ifdef BLENT_AVX2
    return policy == CONV_SIMD_AVX2 && __builtin_cpu_supports("avx2");
#else
    return 0;
#endif
}

int conv_pool_set_simd(conv_pool_t *pool, conv_simd_t policy) {
    if (!conv_simd_available(policy)) return 0;
    pool->simd = policy;
    return 1;
}

typedef struct { int rows; size_t width; int split; } conv_work_t;

#ifdef BLENT_AVX2
static int avx2_scale(int scale) {
    return scale == 1 || scale == 2 || scale == 4;
}

static int use_avx2(const conv_job_t *frame, conv_work_t work, conv_simd_t policy) {
    if (!avx2_scale(frame->scale) || policy == CONV_SIMD_PORTABLE) return 0;
    if (!conv_simd_available(CONV_SIMD_AVX2)) return 0;
    if (policy == CONV_SIMD_AVX2 || frame->scale != 1) return 1;
    if (frame->dirty || frame->spans) return work.rows > 0 && work.width >= (size_t)work.rows * 256;
    return frame->ow >= 256;
}
#endif

static conv_kernel_t select_kernel(const conv_job_t *frame, conv_work_t work, conv_simd_t policy) {
#ifdef BLENT_AVX2
    if (use_avx2(frame, work, policy)) {
        if (frame->order == PIXEL_RGBA) return work.split ? convert_avx2_rgba_adaptive_job : convert_avx2_rgba_job;
        return work.split ? convert_avx2_adaptive_job : convert_avx2_job;
    }
#else
    (void)frame; (void)policy;
#endif
    if (frame->order == PIXEL_RGBA) return work.split ? convert_rgba_adaptive_job : convert_rgba_job;
    return work.split ? convert_adaptive_job : convert_job;
}

static size_t row_work(const conv_job_t *frame, int cy) {
    if (!row_is_dirty(frame->dirty, cy)) return 0;
    pixel_row_t span = row_span(frame, cy);
    size_t width = (size_t)(span.end - span.begin);
    if (pixel_row_split(span)) width -= (size_t)(span.gap.end - span.gap.begin);
    return width;
}

static conv_work_t measure_work(const conv_job_t *frame) {
    conv_work_t work = {0};
    for (int cy = frame->cy0; cy < frame->cy1; cy++) {
        if (!row_is_dirty(frame->dirty, cy)) continue;
        work.rows++;
        size_t width = row_work(frame, cy);
        pixel_row_t row = row_span(frame, cy);
        work.width += width;
        work.split |= width != (size_t)(row.end - row.begin);
    }
    return work;
}

/* Explicit NULL masks retain row-count dispatch for pool measurements and
 * callers supplying empty spans. Capture uses its per-buffer dirty history. */
static int job_count(const conv_pool_t *pool, const conv_job_t *frame, conv_work_t work) {
    if (!frame->dirty) return pool->count;
    size_t pixels = work.width * 2 * frame->scale * frame->scale;
    size_t count = (pixels + 262143) / 262144;
    if (count > (size_t)work.rows) count = (size_t)work.rows;
    if (count > (size_t)pool->count) count = (size_t)pool->count;
    return (int)count;
}

static void *conv_worker(void *arg) {
    conv_worker_arg_t *worker = arg;
    conv_pool_t *pool = worker->pool;
    int id = worker->id;
    for (;;) {
        pthread_mutex_lock(&pool->mutex);
        while (!worker->pending && !pool->shutdown)
            pthread_cond_wait(&worker->ready, &pool->mutex);
        if (pool->shutdown) { pthread_mutex_unlock(&pool->mutex); break; }
        worker->pending = 0;
        pthread_mutex_unlock(&pool->mutex);

        pool->kernel(&pool->jobs[id]);

        pthread_mutex_lock(&pool->mutex);
        if (--pool->active == 0)
            pthread_cond_signal(&pool->done);
        pthread_mutex_unlock(&pool->mutex);
    }
    return NULL;
}

void conv_pool_start(conv_pool_t *pool, int count) {
    pool->count = count < 1 ? 1 : count;
    if (pool->count > MAX_CONV_THREADS) pool->count = MAX_CONV_THREADS;
    /* Worker threads handle jobs 1..n-1; the caller runs job 0 itself. */
    for (int i = 1; i < pool->count; i++) {
        pool->workers[i].pool = pool;
        pool->workers[i].id = i;
        int error = pthread_cond_init(&pool->workers[i].ready, NULL);
        if (error == 0) {
            error = pthread_create(&pool->threads[i], NULL, conv_worker, &pool->workers[i]);
            if (error) pthread_cond_destroy(&pool->workers[i].ready);
        }
        if (error != 0) {
            fprintf(stderr, "[evdi-helper] Conversion worker unavailable: %s\n", strerror(error));
            pool->count = i;
            break;
        }
    }
    fprintf(stderr, "[evdi-helper] NV12 conversion using %d thread(s)\n", pool->count);
}

static int weighted_end(const conv_job_t *frame, int begin, size_t target, size_t *accumulated) {
    while (begin < frame->cy1 && *accumulated < target) {
        *accumulated += row_work(frame, begin);
        begin++;
    }
    return begin;
}

static void prepare_jobs(conv_pool_t *pool, const conv_job_t *frame, int count, conv_work_t work) {
    int begin = frame->cy0;
    size_t accumulated = 0;
    for (int i = 0; i < count; i++) {
        int end = frame->cy0 + (frame->cy1 - frame->cy0) * (i + 1) / count;
        if (frame->dirty) {
            size_t target = (work.width * (i + 1) + count - 1) / count;
            end = weighted_end(frame, begin, target, &accumulated);
        }
        pool->jobs[i] = *frame;
        pool->jobs[i].cy0 = begin;
        pool->jobs[i].cy1 = i == count - 1 ? frame->cy1 : end;
        begin = end;
    }
}

static void start_workers(conv_pool_t *pool, int count) {
    pthread_mutex_lock(&pool->mutex);
    pool->active = count - 1;
    pool->generation++;
    for (int i = 1; i < count; i++) {
        pool->workers[i].pending = 1;
        pthread_cond_signal(&pool->workers[i].ready);
    }
    pthread_mutex_unlock(&pool->mutex);
}

void conv_pool_convert(conv_pool_t *pool, const conv_job_t *frame) {
    if (!pixel_order_valid(frame->order)) { pool->last_jobs = 0; return; }
    conv_work_t work = (frame->dirty || frame->spans) ? measure_work(frame) : (conv_work_t){0};
    pool->kernel = select_kernel(frame, work, pool->simd);
    int count = job_count(pool, frame, work);
    pool->last_jobs = count;
    if (count == 0) return;
    if (count == 1) { pool->kernel(frame); return; }
    prepare_jobs(pool, frame, count, work);
    start_workers(pool, count);
    pool->kernel(&pool->jobs[0]);   /* caller does its own strip */
    pthread_mutex_lock(&pool->mutex);
    while (pool->active > 0)
        pthread_cond_wait(&pool->done, &pool->mutex);
    pthread_mutex_unlock(&pool->mutex);
}

void conv_pool_stop(conv_pool_t *pool) {
    pthread_mutex_lock(&pool->mutex);
    pool->shutdown = 1;
    for (int i = 1; i < pool->count; i++) pthread_cond_signal(&pool->workers[i].ready);
    pthread_mutex_unlock(&pool->mutex);
    for (int i = 1; i < pool->count; i++) {
        pthread_join(pool->threads[i], NULL);
        pthread_cond_destroy(&pool->workers[i].ready);
    }
}

void conv_pool_destroy(conv_pool_t *pool) {
    conv_pool_stop(pool);
    pthread_cond_destroy(&pool->done);
    pthread_mutex_destroy(&pool->mutex);
}
