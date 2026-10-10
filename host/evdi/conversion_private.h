#ifndef BLENT_CONVERSION_PRIVATE_H
#define BLENT_CONVERSION_PRIVATE_H
#include "conversion.h"
#include <pthread.h>

/* Native owner layout; never reexport through the pixel-job contract. */
typedef void (*conv_kernel_t)(const conv_job_t *job);

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

#endif
