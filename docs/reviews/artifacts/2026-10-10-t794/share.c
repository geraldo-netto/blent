/* Disposable T794 native probe. Stock VAAPI objects stay in their process. */
#define MAPPED_LIBRARY
#include "mapped.c"
#include <sys/socket.h>
#include <sys/mman.h>
#include <sys/wait.h>

typedef struct { int sequence, slot; } request;
static void packet_write(int fd, const void *data, size_t bytes) {
    gpu_require(send(fd,data,bytes,MSG_NOSIGNAL)==(ssize_t)bytes,"send message");
}
static void packet_read(int fd, void *data, size_t bytes) {
    gpu_require(recv(fd,data,bytes,0)==(ssize_t)bytes,"receive message");
}
static void descriptor_send(int fd, VADisplay va, AVFrame *frame) {
    VADRMPRIMESurfaceDescriptor desc={0};
    gpu_require(vaExportSurfaceHandle(va,(VASurfaceID)(uintptr_t)frame->data[3],
        VA_SURFACE_ATTRIB_MEM_TYPE_DRM_PRIME_2,VA_EXPORT_SURFACE_READ_ONLY|VA_EXPORT_SURFACE_COMPOSED_LAYERS,&desc)==0,"export NV12");
    gpu_require(desc.num_objects>0 && desc.num_objects<=4,"export FD count");
    char control[CMSG_SPACE(4*sizeof(int))]={0};
    struct iovec iov={&desc,sizeof(desc)};
    struct msghdr msg={.msg_iov=&iov,.msg_iovlen=1,.msg_control=control,.msg_controllen=CMSG_SPACE(desc.num_objects*sizeof(int))};
    struct cmsghdr *header=CMSG_FIRSTHDR(&msg);
    header->cmsg_level=SOL_SOCKET;header->cmsg_type=SCM_RIGHTS;header->cmsg_len=CMSG_LEN(desc.num_objects*sizeof(int));
    for(unsigned i=0;i<desc.num_objects;i++) ((int *)CMSG_DATA(header))[i]=desc.objects[i].fd;
    gpu_require(sendmsg(fd,&msg,MSG_NOSIGNAL)==sizeof(desc),"pass descriptor");
    fprintf(stderr,"export %ux%u objects=%u layers=%u pitch=%u modifier=%"PRIu64"\n",desc.width,desc.height,desc.num_objects,desc.num_layers,desc.layers[0].pitch[0],desc.objects[0].drm_format_modifier);
    for(unsigned i=0;i<desc.num_objects;i++) close(desc.objects[i].fd);
}
static VASurfaceID descriptor_receive(int fd,gpu_encoder *encoder,int width,int height) {
    VADRMPRIMESurfaceDescriptor desc={0};char control[CMSG_SPACE(4*sizeof(int))]={0};
    struct iovec iov={&desc,sizeof(desc)};
    struct msghdr msg={.msg_iov=&iov,.msg_iovlen=1,.msg_control=control,.msg_controllen=sizeof(control)};
    gpu_require(recvmsg(fd,&msg,MSG_CMSG_CLOEXEC)==sizeof(desc),"receive descriptor");
    struct cmsghdr *header=CMSG_FIRSTHDR(&msg);
    gpu_require(header && !(msg.msg_flags & (MSG_TRUNC|MSG_CTRUNC)),"complete descriptor");
    gpu_require(desc.num_objects>0 && desc.num_objects<=4,"received FD count");
    gpu_require(header->cmsg_len==CMSG_LEN(desc.num_objects*sizeof(int)),"descriptor count matches");
    for(unsigned i=0;i<desc.num_objects;i++) desc.objects[i].fd=((int *)CMSG_DATA(header))[i];
    VASurfaceAttrib attributes[2]={0};
    attributes[0]=(VASurfaceAttrib){.type=VASurfaceAttribMemoryType,.flags=VA_SURFACE_ATTRIB_SETTABLE,.value={.type=VAGenericValueTypeInteger,.value.i=VA_SURFACE_ATTRIB_MEM_TYPE_DRM_PRIME_2}};
    attributes[1]=(VASurfaceAttrib){.type=VASurfaceAttribExternalBufferDescriptor,.flags=VA_SURFACE_ATTRIB_SETTABLE,.value={.type=VAGenericValueTypePointer,.value.p=&desc}};
    VASurfaceID surface;
    gpu_require(vaCreateSurfaces(encoder->va,VA_RT_FORMAT_YUV420,width,height,&surface,1,attributes,2)==0,"import NV12");
    for(unsigned i=0;i<desc.num_objects;i++) close(desc.objects[i].fd);
    return surface;
}
static void retired(void *opaque,uint8_t *data) { *(int *)opaque=1;free(data); }
static AVFrame *import_frame(gpu_encoder *encoder,VASurfaceID surface,int width,int height,int *done) {
    AVFrame *frame=av_frame_alloc();assert(frame);
    frame->format=AV_PIX_FMT_VAAPI;frame->width=width;frame->height=height;
    frame->data[3]=(uint8_t *)(uintptr_t)surface;frame->hw_frames_ctx=av_buffer_ref(encoder->frames);
    frame->buf[0]=av_buffer_create(malloc(1),1,retired,done,0);
    gpu_require(frame->hw_frames_ctx && frame->buf[0],"imported reference");return frame;
}
static AVFrame *upload_frame(gpu_encoder *encoder,unsigned char *shared,int width,int height) {
    AVFrame *cpu=cpu_frame(width,height),*hardware=av_frame_alloc();assert(hardware);
    for(int y=0;y<height;y++) memcpy(cpu->data[0]+(size_t)y*cpu->linesize[0],shared+(size_t)y*width,width);
    for(int y=0;y<height/2;y++) memcpy(cpu->data[1]+(size_t)y*cpu->linesize[1],shared+(size_t)(height+y)*width,width);
    gpu_av(av_hwframe_get_buffer(encoder->frames,hardware,0),"baseline surface");
    gpu_av(av_hwframe_transfer_data(hardware,cpu,0),"baseline upload");av_frame_free(&cpu);return hardware;
}
static void child_frame(int fd,gpu_encoder *encoder,VASurfaceID *surfaces,unsigned char *shared,int width,int height,int mode,FILE *output) {
    request req;packet_read(fd,&req,sizeof(req));
    gpu_require(req.slot>=0 && req.slot<3,"slot");int done=0;
    AVFrame *frame=mode?import_frame(encoder,surfaces[req.slot],width,height,&done):upload_frame(encoder,shared,width,height);
    encode_one(encoder,frame,req.sequence,output);av_frame_free(&frame);
    gpu_require(!mode || done,"final input reference retired before reuse");
    gpu_require(vaSyncSurface(encoder->va,surfaces[req.slot])==0,"imported surface completed");
    packet_write(fd,&req,sizeof(req));
}
static void child(int fd,const char *node,int width,int height,int mode,unsigned char *shared,const char *path) {
    alarm(30);gpu_encoder encoder={0};encoder_open(&encoder,node,width,height);
    VASurfaceID surfaces[3];for(int i=0;i<3;i++) surfaces[i]=descriptor_receive(fd,&encoder,width,height);
    FILE *output=fopen(path,"wb");assert(output);int ready=1;packet_write(fd,&ready,sizeof(ready));
    for(int i=0;i<18;i++) child_frame(fd,&encoder,surfaces,shared,width,height,mode,output);
    avcodec_free_context(&encoder.codec);av_packet_free(&encoder.packet);
    gpu_require(vaDestroySurfaces(encoder.va,surfaces,3)==0,"destroy imported surfaces");
    av_buffer_unref(&encoder.frames);av_buffer_unref(&encoder.device);assert(!fclose(output));close(fd);_exit(0);
}
static void patch_plane(uint8_t *plane,int pitch,int width,int height,int sequence,int chroma) {
    int begin=sequence<3?0:(sequence*14)%(height-16);
    int end=sequence<3?height:begin+16;
    int left=sequence<3?0:(sequence*18)%(width-32);
    int right=sequence<3?width:left+32;
    for(int y=begin;y<end;y++) for(int x=left;x<right;x++) plane[(size_t)y*pitch+x]=chroma?(uint8_t)(100+(x+sequence)%50):(uint8_t)(16+(x+y+sequence)%220);
}
static void check_plane(uint8_t *mapped,int pitch,uint8_t *expected,int width,int height) {
    for(int y=0;y<height;y++) { if(memcmp(mapped+(size_t)y*pitch,expected+(size_t)y*width,width)) { fprintf(stderr,"readback row %d: first=%u expected=%u pitch=%d width=%d\n",y,mapped[(size_t)y*pitch],expected[(size_t)y*width],pitch,width);exit(2); } }
}
static void producer_frame(AVFrame *hardware,uint8_t *expected,int width,int height,int sequence) {
    AVFrame *view=av_frame_alloc();assert(view);view->format=AV_PIX_FMT_NV12;
    int flags=AV_HWFRAME_MAP_WRITE|AV_HWFRAME_MAP_DIRECT;
    if(sequence<3) flags|=AV_HWFRAME_MAP_OVERWRITE;
    else flags|=AV_HWFRAME_MAP_READ;
    gpu_av(av_hwframe_map(view,hardware,flags),"map producer");
    patch_plane(view->data[0],view->linesize[0],width,height,sequence,0);
    patch_plane(view->data[1],view->linesize[1],width,height/2,sequence,1);
    patch_plane(expected,width,width,height,sequence,0);
    patch_plane(expected+(size_t)width*height,width,width,height/2,sequence,1);
    av_frame_free(&view);view=av_frame_alloc();assert(view);view->format=AV_PIX_FMT_NV12;
    gpu_av(av_hwframe_map(view,hardware,AV_HWFRAME_MAP_READ|AV_HWFRAME_MAP_DIRECT),"readback producer");
    check_plane(view->data[0],view->linesize[0],expected,width,height);
    check_plane(view->data[1],view->linesize[1],expected+(size_t)width*height,width,height/2);
    av_frame_free(&view);
}
static void producer(int fd,const char *node,int width,int height,unsigned char *shared) {
    gpu_encoder encoder={0};
    gpu_av(av_hwdevice_ctx_create(&encoder.device,AV_HWDEVICE_TYPE_VAAPI,node,NULL,0),"producer device");
    encoder.va=((AVVAAPIDeviceContext *)((AVHWDeviceContext *)encoder.device->data)->hwctx)->display;
    gpu_options options={.width=width,.height=height};frame_pool(&encoder,&options);
    AVFrame *frames[3];size_t bytes=(size_t)width*height*3/2;uint8_t *expected[3];
    for(int i=0;i<3;i++) { frames[i]=av_frame_alloc();assert(frames[i]);gpu_av(av_hwframe_get_buffer(encoder.frames,frames[i],0),"producer surface");expected[i]=calloc(1,bytes);assert(expected[i]);descriptor_send(fd,encoder.va,frames[i]); }
    int ready;packet_read(fd,&ready,sizeof(ready));assert(ready==1);
    for(int i=0;i<18;i++) {
        request req={i,i%3},ack;fprintf(stderr,"sequence=%d slot=%d\n",i,req.slot);producer_frame(frames[req.slot],expected[req.slot],width,height,i);
        memcpy(shared,expected[req.slot],bytes);packet_write(fd,&req,sizeof(req));packet_read(fd,&ack,sizeof(ack));assert(!memcmp(&req,&ack,sizeof(req)));
    }
    for(int i=0;i<3;i++) { av_frame_free(&frames[i]);free(expected[i]); }
    av_buffer_unref(&encoder.frames);av_buffer_unref(&encoder.device);close(fd);
}
#ifndef SHARING_LIBRARY
int main(int argc,char **argv) {
    assert(argc==6);alarm(30);
    int width=benchmark_number(argv[2],128,1920),height=benchmark_number(argv[3],128,1080),mode=benchmark_number(argv[4],0,1);
    assert(!(width%2) && !(height%2));size_t bytes=(size_t)width*height*3/2;
    unsigned char *shared=mmap(NULL,bytes,PROT_READ|PROT_WRITE,MAP_SHARED|MAP_ANONYMOUS,-1,0);assert(shared!=MAP_FAILED);
    int pair[2];assert(!socketpair(AF_UNIX,SOCK_SEQPACKET,0,pair));pid_t pid=fork();assert(pid>=0);
    if(!pid) { close(pair[0]);child(pair[1],argv[1],width,height,mode,shared,argv[5]); }
    close(pair[1]);producer(pair[0],argv[1],width,height,shared);
    int status;assert(waitpid(pid,&status,0)==pid);gpu_require(WIFEXITED(status) && WEXITSTATUS(status)==0,"child success");
    assert(!munmap(shared,bytes));puts("PASS: 3 exported/imported slots, 18 frames, sparse readback, final-reference retirement and device completion");return 0;
}

#endif
