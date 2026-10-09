#define _POSIX_C_SOURCE 200809L
#include <libavutil/hwcontext.h>
#include <libavutil/frame.h>
#include <libavutil/error.h>
#include <stdio.h>
#include <assert.h>
#include <string.h>
static void check(int n) {if(n<0){char b[128];av_strerror(n,b,sizeof(b));fprintf(stderr,"%s\n",b);assert(n>=0);}}
int main(int argc,char **argv) {
 assert(argc==2);AVBufferRef *device=NULL;check(av_hwdevice_ctx_create(&device,AV_HWDEVICE_TYPE_VAAPI,argv[1],NULL,0));
 AVBufferRef *pool=av_hwframe_ctx_alloc(device);assert(pool);AVHWFramesContext *ctx=(void*)pool->data;ctx->format=AV_PIX_FMT_VAAPI;ctx->sw_format=AV_PIX_FMT_NV12;ctx->width=1280;ctx->height=800;ctx->initial_pool_size=4;check(av_hwframe_ctx_init(pool));
 AVFrame *hw=av_frame_alloc(),*sw=av_frame_alloc(),*map=av_frame_alloc();assert(hw&&sw&&map);check(av_hwframe_get_buffer(pool,hw,0));
 sw->format=AV_PIX_FMT_NV12;sw->width=1280;sw->height=800;check(av_frame_get_buffer(sw,32));memset(sw->data[0],64,sw->linesize[0]*800);memset(sw->data[1],128,sw->linesize[1]*400);check(av_hwframe_transfer_data(hw,sw,0));
 map->format=AV_PIX_FMT_NV12;int status=av_hwframe_map(map,hw,AV_HWFRAME_MAP_WRITE|AV_HWFRAME_MAP_OVERWRITE|AV_HWFRAME_MAP_DIRECT);
 char error[128];av_strerror(status,error,sizeof(error));printf("{\"device\":\"%s\",\"upload\":true,\"direct_map_status\":%d,\"description\":\"%s\",\"pitch\":%d}\n",argv[1],status,error,map->linesize[0]);
 av_frame_free(&map);av_frame_free(&sw);av_frame_free(&hw);av_buffer_unref(&pool);av_buffer_unref(&device);return 0;
}
