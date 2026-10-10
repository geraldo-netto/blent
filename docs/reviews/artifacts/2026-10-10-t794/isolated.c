/* T794 timing microbench: two driver-owning processes, bounded 3-slot pool.
 * CPU baseline shares anonymous RAM; this is not the production supervisor.
 * One frame in flight, stock settings identical to mapped.c. */
#define SHARING_LIBRARY
#include "share.c"
typedef struct { uint64_t cpu_ns, upload_ns, encode_ns; } completion;
typedef struct {
    int fd,width,height,mode,frames;const char *node,*path;uint8_t *shared;
} run;
static AVFrame *shared_upload(gpu_encoder *encoder,const run *r) {
    AVFrame cpu={.format=AV_PIX_FMT_NV12,.width=r->width,.height=r->height};
    cpu.data[0]=r->shared;cpu.data[1]=r->shared+(size_t)r->width*r->height;
    cpu.linesize[0]=cpu.linesize[1]=r->width;
    AVFrame *hardware=av_frame_alloc();assert(hardware);
    gpu_av(av_hwframe_get_buffer(encoder->frames,hardware,0),"CPU baseline surface");
    gpu_av(av_hwframe_transfer_data(hardware,&cpu,0),"shared CPU upload");return hardware;
}
static void codec_frame(gpu_encoder *encoder,const run *r,VASurfaceID *surfaces,FILE *output) {
    request req;packet_read(r->fd,&req,sizeof(req));gpu_require(req.slot>=0 && req.slot<3,"timing slot");
    uint64_t cpu_start=stamp(CLOCK_PROCESS_CPUTIME_ID),start=stamp(CLOCK_MONOTONIC);int retired=0;
    AVFrame *frame=r->mode?import_frame(encoder,surfaces[req.slot],r->width,r->height,&retired):shared_upload(encoder,r);
    uint64_t prepared=stamp(CLOCK_MONOTONIC),end=encode_one(encoder,frame,req.sequence,output);
    VASurfaceID surface=(VASurfaceID)(uintptr_t)frame->data[3];
    gpu_require(vaSyncSurface(encoder->va,surface)==0,"completed input surface");
    av_frame_free(&frame);gpu_require(!r->mode || retired,"input reference retirement");
    completion result={stamp(CLOCK_PROCESS_CPUTIME_ID)-cpu_start,prepared-start,end-prepared};
    packet_write(r->fd,&result,sizeof(result));
}
static void codec_process(const run *r) {
    alarm(30);gpu_encoder encoder={0};encoder_open(&encoder,r->node,r->width,r->height);VASurfaceID surfaces[3]={0};
    if(r->mode) for(int i=0;i<3;i++) surfaces[i]=descriptor_receive(r->fd,&encoder,r->width,r->height);
    FILE *output=fopen(r->path,"wb");assert(output);int ready=1;packet_write(r->fd,&ready,sizeof(ready));
    for(int i=0;i<r->frames+20;i++) codec_frame(&encoder,r,surfaces,output);
    avcodec_free_context(&encoder.codec);av_packet_free(&encoder.packet);
    if(r->mode) gpu_require(vaDestroySurfaces(encoder.va,surfaces,3)==0,"destroy timing imports");
    av_buffer_unref(&encoder.frames);av_buffer_unref(&encoder.device);assert(!fclose(output));close(r->fd);_exit(0);
}
static void capture_open(const run *r,gpu_encoder *encoder,AVFrame **surfaces) {
    if(!r->mode) return;
    gpu_av(av_hwdevice_ctx_create(&encoder->device,AV_HWDEVICE_TYPE_VAAPI,r->node,NULL,0),"capture device");
    encoder->va=((AVVAAPIDeviceContext *)((AVHWDeviceContext *)encoder->device->data)->hwctx)->display;
    gpu_options options={.width=r->width,.height=r->height};frame_pool(encoder,&options);
    for(int i=0;i<3;i++) {
        surfaces[i]=av_frame_alloc();assert(surfaces[i]);gpu_av(av_hwframe_get_buffer(encoder->frames,surfaces[i],0),"capture surface");
        descriptor_send(r->fd,encoder->va,surfaces[i]);
    }
}
static void timing_row(int sequence,const uint64_t *times,completion child) {
    if(sequence<20) return;
    printf("%s{\"map_ns\":%"PRIu64",\"conversion_ns\":%"PRIu64",\"unmap_ns\":%"PRIu64",\"upload_ns\":%"PRIu64",\"encode_ns\":%"PRIu64",\"total_ns\":%"PRIu64",\"cpu_ns\":%"PRIu64"}",sequence>20?",":"",times[1]-times[0],times[2]-times[1],times[3]-times[2],child.upload_ns,child.encode_ns,times[4]-times[0],times[5]+child.cpu_ns);
}
static void capture_frame(const run *r,AVFrame **surfaces,conv_pool_t *pool,uint8_t *rgb,int sequence) {
    paint(rgb,r->width,r->height,sequence);uint64_t cpu_start=stamp(CLOCK_PROCESS_CPUTIME_ID),t[6];t[0]=stamp(CLOCK_MONOTONIC);
    AVFrame cpu={.data={r->shared,r->shared+(size_t)r->width*r->height},.linesize={r->width,r->width}};
    AVFrame *destination=r->mode?mapped(surfaces[sequence%3]):&cpu;t[1]=stamp(CLOCK_MONOTONIC);
    convert_rgb(pool,rgb,destination,r->width,r->height);t[2]=stamp(CLOCK_MONOTONIC);
    if(r->mode) av_frame_free(&destination);
    t[3]=stamp(CLOCK_MONOTONIC);request req={sequence,sequence%3};packet_write(r->fd,&req,sizeof(req));
    completion child;packet_read(r->fd,&child,sizeof(child));t[4]=stamp(CLOCK_MONOTONIC);t[5]=stamp(CLOCK_PROCESS_CPUTIME_ID)-cpu_start;
    timing_row(sequence,t,child);struct timespec gap={.tv_nsec=10000000};nanosleep(&gap,NULL);
}
static void capture_process(const run *r) {
    gpu_encoder encoder={0};AVFrame *surfaces[3]={0};capture_open(r,&encoder,surfaces);
    int ready;packet_read(r->fd,&ready,sizeof(ready));assert(ready==1);
    conv_pool_t pool=CONV_POOL_INITIALIZER;conv_pool_start(&pool,4);
    uint8_t *rgb=malloc((size_t)r->width*r->height*4);assert(rgb);
    printf("{\"width\":%d,\"height\":%d,\"mapped\":%d,\"frames\":[",r->width,r->height,r->mode);
    for(int i=0;i<r->frames+20;i++) capture_frame(r,surfaces,&pool,rgb,i);
    puts("]}");conv_pool_destroy(&pool);free(rgb);
    for(int i=0;i<3;i++) av_frame_free(&surfaces[i]);
    av_buffer_unref(&encoder.frames);av_buffer_unref(&encoder.device);close(r->fd);
}
int main(int argc,char **argv) {
    assert(argc==7);alarm(30);run r={.node=argv[1],.path=argv[6]};
    r.width=benchmark_number(argv[2],128,1920);r.height=benchmark_number(argv[3],128,1080);
    r.mode=benchmark_number(argv[4],0,1);r.frames=benchmark_number(argv[5],1,300);
    assert(!(r.width%2) && !(r.height%2));size_t bytes=(size_t)r.width*r.height*3/2;
    r.shared=mmap(NULL,bytes,PROT_READ|PROT_WRITE,MAP_SHARED|MAP_ANONYMOUS,-1,0);assert(r.shared!=MAP_FAILED);
    int pair[2];assert(!socketpair(AF_UNIX,SOCK_SEQPACKET,0,pair));pid_t pid=fork();assert(pid>=0);
    if(!pid) { close(pair[0]);r.fd=pair[1];codec_process(&r); }
    close(pair[1]);r.fd=pair[0];capture_process(&r);int status;assert(waitpid(pid,&status,0)==pid);
    gpu_require(WIFEXITED(status) && WEXITSTATUS(status)==0,"timing child success");assert(!munmap(r.shared,bytes));return 0;
}
