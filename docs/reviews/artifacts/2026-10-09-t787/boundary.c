#define _GNU_SOURCE
#include "conversion.c"
#include "encode.c"
#include "checked_arguments.h"
#include <assert.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
#include <errno.h>
#include <inttypes.h>
#include <fcntl.h>
static uint64_t stamp(void){struct timespec t;assert(!clock_gettime(CLOCK_MONOTONIC,&t));return (uint64_t)t.tv_sec*1000000000+t.tv_nsec;}
void gpu_require(int valid,const char *message){if(!valid){fprintf(stderr,"%s\n",message);abort();}}
static void send_bytes(int fd,const void *data,size_t bytes){const char *p=data;while(bytes){ssize_t n=write(fd,p,bytes);if(n<0&&errno==EINTR)continue;assert(n>0);p+=n;bytes-=n;}}
static void get_bytes(int fd,void *data,size_t bytes){char *p=data;while(bytes){ssize_t n=read(fd,p,bytes);if(n<0&&errno==EINTR)continue;assert(n>0);p+=n;bytes-=n;}}
static void open_encoder(gpu_encoder *e,const char *node){gpu_options options={.width=1280,.height=800,.fps=60,.quality=18,.bitrate=20000};gpu_av(av_hwdevice_ctx_create(&e->device,AV_HWDEVICE_TYPE_VAAPI,node,NULL,0),"device");e->va=((AVVAAPIDeviceContext*)((AVHWDeviceContext*)e->device->data)->hwctx)->display;frame_pool(e,&options);codec_open(e,&options);e->packet=av_packet_alloc();assert(e->packet);}
static uint64_t encode_one(gpu_encoder *e,AVFrame *sw,int sequence,FILE *output){AVFrame *hw=av_frame_alloc();assert(hw);gpu_av(av_hwframe_get_buffer(e->frames,hw,0),"surface");gpu_av(av_hwframe_transfer_data(hw,sw,0),"upload");hw->pts=sequence*16667;gpu_av(avcodec_send_frame(e->codec,hw),"submit");av_frame_free(&hw);gpu_av(avcodec_receive_packet(e->codec,e->packet),"packet");uint64_t end=stamp();assert(fwrite(e->packet->data,1,e->packet->size,output)==(size_t)e->packet->size);av_packet_unref(e->packet);return end;}
static void consume(const char *node,unsigned char *shared,int mode,int input,int reply,FILE *output){enum{N=1280*800*3/2};gpu_encoder e={0};open_encoder(&e,node);unsigned char *pixels=mode?shared:malloc(N);assert(pixels);AVFrame *sw=av_frame_alloc();assert(sw);sw->format=AV_PIX_FMT_NV12;sw->width=1280;sw->height=800;sw->data[0]=pixels;sw->data[1]=pixels+1280*800;sw->linesize[0]=sw->linesize[1]=1280;uint64_t end=0;send_bytes(reply,&end,sizeof(end));
 for(int i=0;i<150;i++){get_bytes(input,&end,sizeof(end));if(!mode)get_bytes(input,pixels,N);end=encode_one(&e,sw,i,output);send_bytes(reply,&end,sizeof(end));}
 av_frame_free(&sw);av_packet_free(&e.packet);avcodec_free_context(&e.codec);av_buffer_unref(&e.frames);av_buffer_unref(&e.device);if(!mode)free(pixels);assert(!fclose(output));close(input);close(reply);_exit(0);}
static void paint(unsigned char *rgb,int sequence){for(int y=0;y<800;y++)for(int x=0;x<1280;x++){unsigned char *p=rgb+(y*1280+x)*4;p[0]=(x+sequence*3)/4;p[1]=(y+sequence*2)/4;p[2]=(x+y+sequence)/8;p[3]=255;}}
int main(int argc,char **argv){assert(argc==5);int mode=benchmark_number(argv[2],0,1),workers=benchmark_number(argv[3],1,4);FILE *output=fopen(argv[4],"wb");assert(output);enum{N=1280*800*3/2};unsigned char *shared=mmap(NULL,N,PROT_READ|PROT_WRITE,MAP_SHARED|MAP_ANONYMOUS,-1,0);assert(shared!=MAP_FAILED);int input[2],reply[2];assert(!pipe(input)&&!pipe(reply));assert(fcntl(input[1],F_SETPIPE_SZ,4<<20)==4<<20);int pipe_bytes=fcntl(input[1],F_GETPIPE_SZ);pid_t pid=fork();assert(pid>=0);alarm(15);
 if(!pid){close(input[1]);close(reply[0]);consume(argv[1],shared,mode,input[0],reply[1],output);}assert(!fclose(output));close(input[0]);close(reply[1]);uint64_t end;get_bytes(reply[0],&end,sizeof(end));
 unsigned char *rgb=malloc(1280*800*4);assert(rgb);conv_pool_t pool=CONV_POOL_INITIALIZER;conv_pool_start(&pool,workers);conv_job_t j={.src=rgb,.ydst=shared,.uvdst=shared+1280*800,.w=1280,.h=800,.stride=1280*4,.ow=1280,.oh=800,.scale=1,.cy1=400};
 printf("{\"device\":\"%s\",\"shared\":%d,\"workers\":%d,\"pipe_bytes\":%d,\"ns\":[",argv[1],mode,workers,pipe_bytes);
 for(int i=0;i<150;i++){paint(rgb,i);uint64_t start=stamp();conv_pool_convert(&pool,&j);send_bytes(input[1],&start,sizeof(start));if(!mode)send_bytes(input[1],shared,N);get_bytes(reply[0],&end,sizeof(end));if(i>=30)printf("%s%"PRIu64,i>30?",":"",end-start);struct timespec gap={.tv_nsec=10000000};nanosleep(&gap,NULL);}
 puts("]}");conv_pool_destroy(&pool);free(rgb);munmap(shared,N);close(input[1]);close(reply[0]);int status;assert(waitpid(pid,&status,0)==pid&&status==0);return 0;}
