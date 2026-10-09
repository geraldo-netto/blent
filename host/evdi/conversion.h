#ifndef BLENT_CONVERSION_H
#define BLENT_CONVERSION_H
#include <pthread.h>
#include <stddef.h>
#include "pixel_span.h"

typedef struct {
    const unsigned char *src;   /* BGRA, stride-padded */
    unsigned char *ydst;        /* Y plane, w bytes/row */
    unsigned char *uvdst;       /* interleaved CbCr, w bytes per chroma row */
    int w, h, stride;           /* source dimensions */
    int ow, oh;                 /* destination dimensions (w/scale, h/scale) */
    int scale;                  /* 1 = no downscale */
    int cy0, cy1;               /* chroma-row range [cy0, cy1) this job owns */
    const unsigned char *dirty; /* NULL = all rows; otherwise one bit/chroma row */
    const pixel_row_t *spans;  /* Optional bounded two-span row history; NULL = full width.
                                * Borrowed, aligned and clipped by frame owner. */
} conv_job_t;

/* AUTO uses available wide kernels only for sufficiently wide unscaled work.
 * PORTABLE retains compiler-vectorized baseline code; AVX2 requires CPU/OS
 * support and forces that unscaled kernel even for short spans. All policies
 * use the same scaled kernels. Unsupported/invalid requests leave state intact. */
typedef enum { CONV_SIMD_AUTO, CONV_SIMD_PORTABLE, CONV_SIMD_AVX2 } conv_simd_t;

typedef void (*conv_kernel_t)(const conv_job_t *job);

#define MAX_CONV_THREADS 128
typedef struct conv_pool conv_pool_t;
typedef struct {
    conv_pool_t *pool;
    int id, pending;
    pthread_cond_t ready;
} conv_worker_arg_t;
struct conv_pool {
    int count, last_jobs;
    conv_simd_t simd;
    conv_kernel_t kernel; /* selected before publishing worker jobs */
    pthread_t threads[MAX_CONV_THREADS];
    conv_job_t jobs[MAX_CONV_THREADS];
    conv_worker_arg_t workers[MAX_CONV_THREADS];
    pthread_mutex_t mutex;
    pthread_cond_t done;
    unsigned generation;
    int active, shutdown;
};
#define CONV_POOL_INITIALIZER { .count = 1, .last_jobs = 1, .mutex = PTHREAD_MUTEX_INITIALIZER, \
    .done = PTHREAD_COND_INITIALIZER }

/* Initialize with CONV_POOL_INITIALIZER; do not move after start. One capture
 * thread owns submission. convert() joins every job before returning, so input,
 * output and dirty masks are borrowed only for the duration of that call.
 * Native/scaled kernels retain distinct hot loops. Destroy after final convert. */
/* Copy nonoverlapping borrowed RGBA pixels to BGRA; callers validate lengths.
 * Alpha/X bytes are preserved. Duplicate damage may safely recopy native pixels. */
void conv_copy_rgba_to_bgra(unsigned char *destination, const unsigned char *source, size_t pixels);
int conv_simd_available(conv_simd_t policy);
/* Owner-thread only, before submission or after convert() has joined jobs. */
int conv_pool_set_simd(conv_pool_t *pool, conv_simd_t policy);
void conv_pool_start(conv_pool_t *pool, int count);
void conv_pool_convert(conv_pool_t *pool, const conv_job_t *frame);
void conv_pool_stop(conv_pool_t *pool);
void conv_pool_destroy(conv_pool_t *pool);
#endif
