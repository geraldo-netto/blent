#define _GNU_SOURCE
#include "conversion.c"
#include "encode.c"
#include "checked_arguments.h"
#include <assert.h>
#include <inttypes.h>
#include <time.h>
#include <unistd.h>

static uint64_t stamp(clockid_t clock) {
    struct timespec value;
    assert(clock_gettime(clock, &value) == 0);
    return (uint64_t)value.tv_sec * 1000000000 + value.tv_nsec;
}
void gpu_require(int valid, const char *message) {
    if (!valid) { fprintf(stderr, "%s\n", message); exit(2); }
}
static void encoder_open(gpu_encoder *encoder, const char *node, int width, int height) {
    gpu_options options = {.width=width, .height=height, .fps=60, .quality=18, .bitrate=20000};
    gpu_av(av_hwdevice_ctx_create(&encoder->device, AV_HWDEVICE_TYPE_VAAPI, node, NULL, 0), "device");
    encoder->va = ((AVVAAPIDeviceContext *)((AVHWDeviceContext *)encoder->device->data)->hwctx)->display;
    frame_pool(encoder, &options); codec_open(encoder, &options);
    encoder->packet = av_packet_alloc(); assert(encoder->packet);
}
static AVFrame *cpu_frame(int width, int height) {
    AVFrame *frame = av_frame_alloc(); assert(frame);
    frame->format = AV_PIX_FMT_NV12; frame->width=width; frame->height=height;
    gpu_av(av_frame_get_buffer(frame, 32), "CPU frame");
    return frame;
}
static void paint(unsigned char *rgb, int width, int height, int sequence) {
    for (int y=0;y<height;y++) for (int x=0;x<width;x++) {
        unsigned char *pixel=rgb+((size_t)y*width+x)*4;
        pixel[0]=(x+sequence*3)/4; pixel[1]=(y+sequence*2)/4;
        pixel[2]=(x+y+sequence)/8; pixel[3]=255;
    }
}
static void convert_rgb(conv_pool_t *pool, unsigned char *rgb, AVFrame *destination, int width, int height) {
    gpu_require(destination->linesize[0]==width && destination->linesize[1]==width, "initial probe requires packed visible output");
    conv_job_t job = {.src=rgb, .ydst=destination->data[0], .uvdst=destination->data[1],
        .w=width, .h=height, .stride=width*4, .ow=width, .oh=height, .scale=1, .cy1=height/2};
    conv_pool_convert(pool, &job);
}
static AVFrame *mapped(AVFrame *hardware) {
    AVFrame *frame=av_frame_alloc();assert(frame);frame->format=AV_PIX_FMT_NV12;
    gpu_av(av_hwframe_map(frame,hardware,AV_HWFRAME_MAP_WRITE|AV_HWFRAME_MAP_OVERWRITE|AV_HWFRAME_MAP_DIRECT),"direct write map");
    return frame;
}
static uint64_t encode_one(gpu_encoder *encoder, AVFrame *frame, int sequence, FILE *output) {
    frame->pts=(int64_t)sequence*16667;
    gpu_av(avcodec_send_frame(encoder->codec,frame),"submit");
    gpu_av(avcodec_receive_packet(encoder->codec,encoder->packet),"packet");
    uint64_t result=stamp(CLOCK_MONOTONIC);
    assert(fwrite(encoder->packet->data,1,encoder->packet->size,output)==(size_t)encoder->packet->size);
    av_packet_unref(encoder->packet);return result;
}
static void replay(gpu_encoder *encoder, int width, int height, int mode, int frames, FILE *output) {
    unsigned char *rgb=malloc((size_t)width*height*4);assert(rgb);
    AVFrame *cpu=cpu_frame(width,height);conv_pool_t pool=CONV_POOL_INITIALIZER;
    conv_pool_start(&pool,4);
    for(int sequence=0;sequence<frames+20;sequence++) {
        paint(rgb,width,height,sequence);
        uint64_t cpu_start=stamp(CLOCK_PROCESS_CPUTIME_ID), start=stamp(CLOCK_MONOTONIC);
        AVFrame *hardware=av_frame_alloc();assert(hardware);
        gpu_av(av_hwframe_get_buffer(encoder->frames,hardware,0),"surface");
        AVFrame *destination=mode?mapped(hardware):cpu;
        uint64_t ready=stamp(CLOCK_MONOTONIC);
        convert_rgb(&pool,rgb,destination,width,height);
        uint64_t converted=stamp(CLOCK_MONOTONIC);
        if(mode) av_frame_free(&destination);
        else gpu_av(av_hwframe_transfer_data(hardware,cpu,0),"upload");
        uint64_t prepared=stamp(CLOCK_MONOTONIC);
        uint64_t end=encode_one(encoder,hardware,sequence,output);
        av_frame_free(&hardware);
        uint64_t cpu_end=stamp(CLOCK_PROCESS_CPUTIME_ID);
        if(sequence>=20) printf("%s{\"map_ns\":%"PRIu64",\"conversion_ns\":%"PRIu64",\"upload_unmap_ns\":%"PRIu64",\"encode_ns\":%"PRIu64",\"total_ns\":%"PRIu64",\"cpu_ns\":%"PRIu64"}",sequence>20?",":"",ready-start,converted-ready,prepared-converted,end-prepared,end-start,cpu_end-cpu_start);
        struct timespec gap={.tv_nsec=10000000};nanosleep(&gap,NULL);
    }
    conv_pool_destroy(&pool);av_frame_free(&cpu);free(rgb);
}
#ifndef MAPPED_LIBRARY
int main(int argc,char **argv) {
    assert(argc==7);alarm(30);
    int width=benchmark_number(argv[2],128,1920),height=benchmark_number(argv[3],128,1080);
    assert(width%2==0 && height%2==0);
    int mode=benchmark_number(argv[4],0,1),frames=benchmark_number(argv[5],1,300);
    gpu_encoder encoder={0};encoder_open(&encoder,argv[1],width,height);
    FILE *output=fopen(argv[6],"wb");assert(output);
    printf("{\"width\":%d,\"height\":%d,\"mapped\":%d,\"frames\":[",width,height,mode);
    replay(&encoder,width,height,mode,frames,output);puts("]}");
    av_packet_free(&encoder.packet);avcodec_free_context(&encoder.codec);
    av_buffer_unref(&encoder.frames);av_buffer_unref(&encoder.device);assert(!fclose(output));return 0;
}

#endif
