/* T768/T769: isolated hypotheses, not production replacements. No devices. */
#define _POSIX_C_SOURCE 200809L
#include "checked_arguments.h"
#include "../../host/evdi/conversion.c"
#include "../../host/evdi/pixel_damage.h"
#include <assert.h>
#include <inttypes.h>
#include <time.h>

enum { W=1920, H=1080, ROWS=H/2, SAMPLES=96, TILE=32 };
struct rectangle { int x0,y0,x1,y1; };
struct replay {
    conv_job_t job;
    unsigned char *source,*output,*expected;
    unsigned char dirty[(ROWS+7)/8];
    pixel_span_t spans[ROWS];
    uint64_t tiles[ROWS];
    struct rectangle rectangles[64];
    int rectangles_count;
};
static volatile size_t observed;
static uint64_t now(clockid_t clock) {
    struct timespec t; assert(!clock_gettime(clock,&t));
    return (uint64_t)t.tv_sec*1000000000+t.tv_nsec;
}
static int compare(const void *a,const void *b) {
    uint64_t x=*(const uint64_t *)a,y=*(const uint64_t *)b;
    return (x>y)-(x<y);
}
static void add_rectangle(struct replay *r,int x0,int y0,int x1,int y1) {
    assert(r->rectangles_count<64);
    r->rectangles[r->rectangles_count++]=(struct rectangle){x0,y0,x1,y1};
}
static void workload(struct replay *r,int kind) {
    int w=r->job.ow,h=r->job.oh;
    switch (kind) {
    case 0: break;
    case 1: add_rectangle(r,0,100,16,164); add_rectangle(r,w-16,100,w,164); break;
    case 2: add_rectangle(r,0,0,w,h); break;
    case 3: add_rectangle(r,0,0,w,(h/4)*2); add_rectangle(r,0,(h/4)*2,32,h); break;
    case 4: add_rectangle(r,64,0,80,h); break;
    case 5:
        for(int x=0;x<w;x+=64) add_rectangle(r,x,100,x+2,164);
        break;
    }
}
static void mark_spans(struct replay *r) {
    memset(r->dirty,0,sizeof(r->dirty));
    for(int i=0;i<r->rectangles_count;i++) {
        struct rectangle b=r->rectangles[i];
        pixel_damage_region(r->dirty,r->spans,(pixel_span_t){b.y0/2,b.y1/2},(pixel_span_t){b.x0,b.x1});
    }
}
static void mark_tiles(struct replay *r) {
    memset(r->tiles,0,sizeof(r->tiles));
    for(int i=0;i<r->rectangles_count;i++) {
        struct rectangle b=r->rectangles[i];
        int first=b.x0/TILE,end=(b.x1+TILE-1)/TILE;
        uint64_t mask=((UINT64_C(1)<<end)-1)^((UINT64_C(1)<<first)-1);
        for(int cy=b.y0/2;cy<b.y1/2;cy++) r->tiles[cy]|=mask;
    }
}
static void paint(struct replay *r) {
    int scale=r->job.scale;
    for(int i=0;i<r->rectangles_count;i++) {
        struct rectangle b=r->rectangles[i];
        for(int y=b.y0*scale;y<b.y1*scale;y++) {
            for(int x=b.x0*scale*4;x<b.x1*scale*4;x++)
                r->source[(size_t)y*r->job.stride+x]=(unsigned char)(x*37+y*53+17);
        }
    }
}
/* Independent scalar oracle: division with explicit negative rounding. */
static int floor256(int value) { return value>=0 ? value/256 : -((-value+255)/256); }
static void oracle_pixel(const conv_job_t *j,int x,int y,int rgb[3]) {
    memset(rgb,0,3*sizeof(int));
    for(int dy=0;dy<j->scale;dy++) {
        for(int dx=0;dx<j->scale;dx++) {
            const unsigned char *p=j->src+(size_t)(y*j->scale+dy)*j->stride+4*(x*j->scale+dx);
            for(int k=0;k<3;k++) rgb[k]+=p[k];
        }
    }
    for(int k=0;k<3;k++) rgb[k]/=j->scale*j->scale;
}
static void oracle_block(struct replay *r,int x,int cy) {
    const conv_job_t *j=&r->job;
    int sum[3]={0};
    for(int q=0;q<4;q++) {
        int rgb[3],px=x+q%2,py=2*cy+q/2;
        oracle_pixel(j,px,py,rgb);
        r->expected[py*j->ow+px]=(47*rgb[2]+157*rgb[1]+16*rgb[0]+128)/256+16;
        for(int k=0;k<3;k++) sum[k]+=rgb[k];
    }
    for(int k=0;k<3;k++) sum[k]/=4;
    unsigned char *uv=r->expected+j->ow*j->oh+cy*j->ow+x;
    uv[0]=floor256(-26*sum[2]-86*sum[1]+112*sum[0]+128)+128;
    uv[1]=floor256(112*sum[2]-102*sum[1]-10*sum[0]+128)+128;
}
static void initialize(struct replay *r,int scale,int kind) {
    int ow=W/scale,oh=H/scale;
    r->source=calloc((W*4+64)*H,1); r->output=malloc(ow*oh*3/2); r->expected=malloc(ow*oh*3/2);
    assert(r->source && r->output && r->expected);
    r->job=(conv_job_t){.src=r->source,.ydst=r->output,.uvdst=r->output+ow*oh,
        .w=W,.h=H,.stride=W*4+64,.ow=ow,.oh=oh,.scale=scale,.cy1=oh/2,.dirty=r->dirty,.spans=r->spans};
    workload(r,kind); paint(r); mark_spans(r); mark_tiles(r);
    for(int cy=0;cy<oh/2;cy++) for(int x=0;x<ow;x+=2) oracle_block(r,x,cy);
}
static void reset_output(struct replay *r) {
    memset(r->output,16,r->job.ow*r->job.oh);
    memset(r->job.uvdst,128,r->job.ow*r->job.oh/2);
}
static void convert_tiles(struct replay *r) {
    conv_job_t job=r->job;
    pixel_span_t spans[ROWS]; job.spans=spans; job.dirty=NULL;
    for(int cy=0;cy<job.oh/2;cy++) {
        uint64_t bits=r->tiles[cy];
        while(bits) {
            int first=__builtin_ctzll(bits),end=first;
            while(end<job.ow/TILE && (bits&(UINT64_C(1)<<end))) end++;
            spans[cy]=(pixel_span_t){first*TILE,end*TILE};
            bits&=~(((UINT64_C(1)<<end)-1)^((UINT64_C(1)<<first)-1));
            job.cy0=cy;job.cy1=cy+1;convert_job(&job);
        }
    }
}
static size_t row_work(const conv_job_t *j,int row) {
    if(!row_is_dirty(j->dirty,row)) return 0;
    pixel_span_t span=row_span(j,row);return (size_t)(span.end-span.begin);
}
static int fused_count(const conv_pool_t *pool,const conv_job_t *j,size_t *width) {
    int rows=0;*width=0;
    for(int cy=j->cy0;cy<j->cy1;cy++) { size_t work=row_work(j,cy); rows+=!!work; *width+=work; }
    size_t count=(*width*2*j->scale*j->scale+262143)/262144;
    if(count>(size_t)rows) count=rows;
    if(count>(size_t)pool->count) count=pool->count;
    return (int)count;
}
/* Experimental contiguous weighted partition; same bounded jobs/owner contract. */
static void weighted_jobs(conv_pool_t *pool,const conv_job_t *j,int count,size_t width) {
    int begin=j->cy0,end=begin;size_t cumulative=0;
    for(int i=0;i<count;i++) {
        size_t target=(width*(i+1)+count-1)/count;
        while(end<j->cy1 && cumulative<target) cumulative+=row_work(j,end++);
        pool->jobs[i]=*j;pool->jobs[i].cy0=begin;
        pool->jobs[i].cy1=i==count-1 ? j->cy1 : end;begin=end;
    }
}
static int schedule(conv_pool_t *pool,const conv_job_t *j,int weighted) {
    size_t width=0;
    int count=weighted ? fused_count(pool,j,&width) : job_count(pool,j);
    if(count<2) return count;
    if(weighted) weighted_jobs(pool,j,count,width); else prepare_jobs(pool,j,count);
    return count;
}
static void weighted_convert(conv_pool_t *pool,const conv_job_t *j) {
    int count=schedule(pool,j,1);pool->last_jobs=count;
    if(!count) return;
    if(count==1) { convert_job(j);return; }
    start_workers(pool,count);convert_job(&pool->jobs[0]);
    pthread_mutex_lock(&pool->mutex);
    while(pool->active>0) pthread_cond_wait(&pool->done,&pool->mutex);
    pthread_mutex_unlock(&pool->mutex);
}
static void execute(struct replay *r,conv_pool_t *pool,int variant) {
    if(variant==1) convert_tiles(r);
    else if(variant==2) weighted_convert(pool,&r->job);
    else conv_pool_convert(pool,&r->job);
}
static void distribution(conv_pool_t *pool,const conv_job_t *j,int variant) {
    int count=schedule(pool,j,variant==2);
    printf(",\"job_source_pixels\":[");
    for(int i=0;i<count;i++) {
        const conv_job_t *part=count==1 ? j : &pool->jobs[i];
        printf("%s%zu",i ? "," : "",dirty_source_pixels(part));
    }
    printf("]");
}
static size_t converted_pixels(const struct replay *r,int variant) {
    if(variant!=1) return dirty_source_pixels(&r->job);
    size_t width=0;
    for(int cy=0;cy<r->job.cy1;cy++) width+=(size_t)__builtin_popcountll(r->tiles[cy])*TILE;
    return width*2*r->job.scale*r->job.scale;
}
static void measure(struct replay *r,conv_pool_t *pool,int variant,int kind) {
    uint64_t samples[SAMPLES],marking[SAMPLES];
    reset_output(r);execute(r,pool,variant);
    assert(!memcmp(r->output,r->expected,r->job.ow*r->job.oh*3/2));
    for(int i=0;i<16;i++) execute(r,pool,variant);
    uint64_t cpu=now(CLOCK_PROCESS_CPUTIME_ID);
    for(int i=0;i<SAMPLES;i++) {
        uint64_t start=now(CLOCK_MONOTONIC);
        if(variant==1) mark_tiles(r);else mark_spans(r);
        marking[i]=now(CLOCK_MONOTONIC)-start;start=now(CLOCK_MONOTONIC);
        execute(r,pool,variant);samples[i]=now(CLOCK_MONOTONIC)-start;
    }
    cpu=now(CLOCK_PROCESS_CPUTIME_ID)-cpu;
    qsort(samples,SAMPLES,sizeof(*samples),compare);qsort(marking,SAMPLES,sizeof(*marking),compare);
    printf("{\"kind\":%d,\"scale\":%d,\"workers\":%d,\"variant\":%d,\"samples\":%d,\"convert_p50_ns\":%"PRIu64",\"convert_p95_ns\":%"PRIu64",\"mark_p50_ns\":%"PRIu64",\"cpu_ns_per_frame\":%"PRIu64,
        kind,r->job.scale,pool->count,variant,SAMPLES,samples[SAMPLES/2],samples[SAMPLES*95/100],marking[SAMPLES/2],cpu/SAMPLES);
    printf(",\"converted_source_pixels\":%zu",converted_pixels(r,variant));
    if(variant!=1) distribution(pool,&r->job,variant);
    puts("}");
}
static void measure_schedule(struct replay *r,conv_pool_t *pool,int kind,int variant) {
    uint64_t samples[SAMPLES];
    for(int i=0;i<SAMPLES;i++) {
        uint64_t start=now(CLOCK_MONOTONIC);
        for(int n=0;n<64;n++) {
            __asm__ volatile("" ::: "memory");
            observed+=(size_t)schedule(pool,&r->job,variant==2);
        }
        samples[i]=(now(CLOCK_MONOTONIC)-start)/64;
    }
    qsort(samples,SAMPLES,sizeof(*samples),compare);
    printf("{\"kind\":%d,\"scale\":%d,\"workers\":%d,\"variant\":%d,\"schedule_p50_ns\":%"PRIu64"}\n",
        kind,r->job.scale,pool->count,variant,samples[SAMPLES/2]);
}
int main(int argc,char **argv) {
    assert(argc==5);
    int scale=benchmark_number(argv[1],1,2),kind=benchmark_number(argv[2],0,5);
    int workers=benchmark_number(argv[3],1,8),variant=benchmark_number(argv[4],0,2);
    assert(variant!=1 || workers==1);
    struct replay r={0};initialize(&r,scale,kind);
    conv_pool_t pool=CONV_POOL_INITIALIZER;conv_pool_start(&pool,workers);
    measure(&r,&pool,variant,kind);
    if(variant!=1) measure_schedule(&r,&pool,kind,variant);
    conv_pool_destroy(&pool);free(r.source);free(r.output);free(r.expected);return 0;
}
