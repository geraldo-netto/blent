#define _POSIX_C_SOURCE 200809L
#include "cursor_overlay.c"
#include "checked_arguments.h"
#include <assert.h>
#include <time.h>
#include <inttypes.h>
static volatile unsigned seen;
static uint64_t stamp(void) {struct timespec t;assert(!clock_gettime(CLOCK_MONOTONIC,&t));return (uint64_t)t.tv_sec*1000000000+t.tv_nsec;}
static void once(unsigned n) {
 size_t size=(size_t)n*n*4;unsigned char *pixels=malloc(size);assert(pixels);memset(pixels,127,size);
 cursor_overlay_t cursor={0};cursor_image_t image={pixels,size,n,n,n*4,1,0};
#ifdef TAKE
 assert(cursor_overlay_take(&cursor,&image));
#else
 assert(cursor_overlay_set(&cursor,&image));free(pixels);
#endif
 seen+=cursor.storage[0];cursor_overlay_destroy(&cursor);
}
int main(int argc,char **argv) {
 assert(argc==2);unsigned n=benchmark_number(argv[1],1,1024);unsigned count=10000000/(n*n)+100;
 for(unsigned i=0;i<64;i++)once(n);
 uint64_t start=stamp();for(unsigned i=0;i<count;i++)once(n);uint64_t elapsed=stamp()-start;
 printf("{\"width\":%u,\"iterations\":%u,\"ns\":%.3f}\n",n,count,(double)elapsed/count);return seen==UINT32_MAX;
}
