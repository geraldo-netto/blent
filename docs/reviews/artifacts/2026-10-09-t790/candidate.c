#define _POSIX_C_SOURCE 200809L
#include "conversion.c"
#include "cursor_overlay.c"
#include "checked_arguments.h"
#include <assert.h>
#include <inttypes.h>
#include <time.h>
#ifdef FUSED_AVX2
#define ISA __attribute__((target("avx2")))
#else
#define ISA
#endif
struct fixture {conv_job_t job;conv_pool_t pool;cursor_overlay_t cursor;unsigned char *source,*pixels,*output,*expected;pixel_row_t spans[540];};
static uint64_t stamp(void){struct timespec t;assert(!clock_gettime(CLOCK_MONOTONIC,&t));return (uint64_t)t.tv_sec*1000000000+t.tv_nsec;}
static inline ISA __attribute__((always_inline)) void blend_pixel(const struct fixture *f,int x,int y,int rgb[3]){
 const unsigned char *source=f->source+(size_t)y*f->job.stride+x*4;
 const unsigned char *cursor=f->pixels+((size_t)(y-16)*f->cursor.width+x-64)*4;unsigned alpha=cursor[3];
 if(!alpha){for(int k=0;k<3;k++)rgb[k]=source[k];return;}
 for(int k=0;k<3;k++){unsigned v=cursor[k]+(source[k]*(255-alpha)+127)/255;rgb[k]=v>255?255:v;}
}
static inline ISA __attribute__((always_inline)) void mean(const struct fixture *f,int x,int y,int rgb[3],int n){
 memset(rgb,0,3*sizeof(int));
 for(int dy=0;dy<n;dy++)for(int dx=0;dx<n;dx++){int p[3];blend_pixel(f,x*n+dx,y*n+dy,p);for(int k=0;k<3;k++)rgb[k]+=p[k];}
 for(int k=0;k<3;k++)rgb[k]/=n*n;
}
static inline ISA __attribute__((always_inline)) void block(struct fixture *f,int x,int cy,int n){
 int sum[3]={0};for(int q=0;q<4;q++){int rgb[3],px=x+q%2,py=2*cy+q/2;mean(f,px,py,rgb,n);f->output[py*f->job.ow+px]=(47*rgb[2]+157*rgb[1]+16*rgb[0]+128)/256+16;for(int k=0;k<3;k++)sum[k]+=rgb[k];}
 write_chroma(f->job.uvdst+cy*f->job.ow+x,sum[0],sum[1],sum[2]);
}
static inline ISA __attribute__((always_inline)) void fused_scale(struct fixture *f,int n){for(int cy=f->job.cy0;cy<f->job.cy1;cy++)for(int x=f->spans[cy].begin;x<f->spans[cy].end;x+=2)block(f,x,cy,n);}
static ISA void fused(struct fixture *f){if(f->job.scale==1)fused_scale(f,1);else if(f->job.scale==2)fused_scale(f,2);else fused_scale(f,4);}
static void baseline(struct fixture *f){cursor_overlay_restore(&f->cursor,f->source,f->job.stride);cursor_overlay_draw(&f->cursor,f->source,1920,1080,f->job.stride);conv_pool_convert(&f->pool,&f->job);}
static void shape_cursor(struct fixture *f,int size,int arrow){
 if(arrow)for(int y=0;y<size;y++)for(int x=0;x<size;x++)if(x>y || x>size/3)memset(f->pixels+(y*size+x)*4,0,4);
}
static void init(struct fixture *f,int n,int size,int arrow){
 f->source=malloc(1920*1080*4);f->pixels=malloc(size*size*4);int ow=1920/n,oh=1080/n;f->output=malloc(ow*oh*3/2);f->expected=malloc(ow*oh*3/2);assert(f->source&&f->pixels&&f->output&&f->expected);
 memset(f->source,63,1920*1080*4);memset(f->pixels,127,size*size*4);
 shape_cursor(f,size,arrow);memset(f->output,0,ow*oh*3/2);cursor_image_t image={f->pixels,(size_t)size*size*4,size,size,size*4,1,0};assert(cursor_overlay_set(&f->cursor,&image));f->cursor.x=64;f->cursor.y=16;
 for(int cy=0;cy<540;cy++)f->spans[cy]=(pixel_row_t){.begin=64/n,.end=(64+size)/n};f->job=(conv_job_t){.src=f->source,.ydst=f->output,.uvdst=f->output+ow*oh,.w=1920,.h=1080,.stride=1920*4,.ow=ow,.oh=oh,.scale=n,.cy0=16/n/2,.cy1=(16+size)/n/2,.spans=f->spans};
 conv_pool_start(&f->pool,1);baseline(f);memcpy(f->expected,f->output,ow*oh*3/2);cursor_overlay_restore(&f->cursor,f->source,f->job.stride);memset(f->output,0,ow*oh*3/2);fused(f);assert(!memcmp(f->expected,f->output,ow*oh*3/2));
}
int main(int argc,char **argv){assert(argc==5);int n=benchmark_number(argv[1],1,4),size=benchmark_number(argv[2],16,1024),use_fused=benchmark_number(argv[3],0,1);assert(n!=3 && size%8==0);int arrow=benchmark_number(argv[4],0,1);struct fixture f={.pool=CONV_POOL_INITIALIZER};init(&f,n,size,arrow);void(*run)(struct fixture*)=use_fused?fused:baseline;
 for(int i=0;i<16;i++)run(&f);uint64_t start=stamp();for(int i=0;i<256;i++)run(&f);printf("{\"scale\":%d,\"size\":%d,\"fused\":%d,\"arrow\":%d,\"ns\":%.3f}\n",n,size,use_fused,arrow,(double)(stamp()-start)/256);
 conv_pool_destroy(&f.pool);cursor_overlay_destroy(&f.cursor);free(f.source);free(f.pixels);free(f.output);free(f.expected);return 0;}
