/* T766: execute Blent's native orchestration with a private X server and
 * deterministic DRI3/VA/codec boundaries. No real render node is opened. */
#define _POSIX_C_SOURCE 200809L
#include "../gpu/gpu.h"
#include <assert.h>
#include <fcntl.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/wait.h>
#include <X11/Xatom.h>
#include <xcb/dri3.h>
#include <va/va_drmcommon.h>
#include <va/va_vpp.h>
#include <libavutil/hwcontext_vaapi.h>
#include <libavutil/buffer.h>
#include <libavutil/opt.h>
#define main gpu_helper_main
#include "../gpu/main.c"
#undef main

static int exported_fd, producer_fd, pending_packet, synchronized;
static int fault, pictures, imports, conversions, submitted;
static int transition, copied, after_copy;

static int changed(int kind) {
    return transition == kind && copied == after_copy;
}

int __real_XCopyArea(Display *, Drawable, Drawable, GC, int, int, unsigned, unsigned, int, int);
int __wrap_XCopyArea(Display *d, Drawable src, Drawable dst, GC gc, int x, int y, unsigned w, unsigned h, int dx, int dy) {
    int result=__real_XCopyArea(d,src,dst,gc,x,y,w,h,dx,dy);
    copied=1; return result;
}

XRROutputInfo *__real_XRRGetOutputInfo(Display *, XRRScreenResources *, RROutput);
XRROutputInfo *__wrap_XRRGetOutputInfo(Display *d, XRRScreenResources *r, RROutput output) {
    XRROutputInfo *info=__real_XRRGetOutputInfo(d,r,output); assert(info);
    if (changed(2)) info->crtc++;
    if (changed(3)) info->connection=RR_Disconnected;
    if (changed(4)) info->crtc=None;
    return info;
}

XRRCrtcInfo *__real_XRRGetCrtcInfo(Display *, XRRScreenResources *, RRCrtc);
XRRCrtcInfo *__wrap_XRRGetCrtcInfo(Display *d, XRRScreenResources *r, RRCrtc crtc) {
    XRRCrtcInfo *info=__real_XRRGetCrtcInfo(d,r,crtc); assert(info);
    if (changed(5)) info->rotation=RR_Rotate_90;
    if (changed(6)) info->x++;
    if (changed(7)) info->y++;
    if (changed(8)) info->width-=2;
    if (changed(9)) info->height-=2;
    return info;
}
static unsigned char identity[128] = {0,255,255,255,255,255,255,0};

xcb_dri3_open_cookie_t __wrap_xcb_dri3_open(xcb_connection_t *c, xcb_drawable_t root, uint32_t provider) {
    (void)c; (void)root; (void)provider; return (xcb_dri3_open_cookie_t){1};
}
xcb_dri3_open_reply_t *__wrap_xcb_dri3_open_reply(xcb_connection_t *c, xcb_dri3_open_cookie_t cookie, xcb_generic_error_t **error) {
    (void)c; (void)cookie; (void)error;
    xcb_dri3_open_reply_t *reply=calloc(1,sizeof(*reply)); assert(reply);
    reply->nfd=fault==4 ? 0 : 1; producer_fd=open("/dev/null",O_RDWR); assert(producer_fd>=0); return reply;
}
int *__wrap_xcb_dri3_open_reply_fds(xcb_connection_t *c, xcb_dri3_open_reply_t *reply) {
    (void)c; (void)reply; return &producer_fd;
}
xcb_dri3_buffer_from_pixmap_cookie_t __wrap_xcb_dri3_buffer_from_pixmap(xcb_connection_t *c, xcb_pixmap_t pixmap) {
    (void)c; (void)pixmap; return (xcb_dri3_buffer_from_pixmap_cookie_t){1};
}
xcb_dri3_buffer_from_pixmap_reply_t *__wrap_xcb_dri3_buffer_from_pixmap_reply(xcb_connection_t *c, xcb_dri3_buffer_from_pixmap_cookie_t cookie, xcb_generic_error_t **error) {
    (void)c; (void)cookie; (void)error;
    xcb_dri3_buffer_from_pixmap_reply_t *reply=calloc(1,sizeof(*reply)); assert(reply);
    reply->nfd=fault==5 ? 0 : 1;
    reply->width=fault==6 ? 127 : 128; reply->height=fault==7 ? 127 : 128;
    reply->depth=fault==8 ? 32 : 24; reply->bpp=fault==9 ? 24 : 32;
    reply->stride=fault==1 ? 1 : 512; reply->size=fault==10 ? 1 : 65536;
    exported_fd=open("/dev/null",O_RDWR); assert(exported_fd>=0); return reply;
}
int *__wrap_xcb_dri3_buffer_from_pixmap_reply_fds(xcb_connection_t *c, xcb_dri3_buffer_from_pixmap_reply_t *reply) {
    (void)c; (void)reply; return &exported_fd;
}
Status __wrap_XRRGetCrtcTransform(Display *display, RRCrtc crtc, XRRCrtcTransformAttributes **attributes) {
    (void)display; (void)crtc;
    *attributes=calloc(1,sizeof(**attributes)); assert(*attributes);
    for (int i=0;i<3;i++) (*attributes)->currentTransform.matrix[i][i]=65536;
    if (fault==2 || changed(10)) (*attributes)->currentTransform.matrix[0][1]=1;
    return 1;
}
int __wrap_XRRGetOutputProperty(Display *d, RROutput output, Atom property, long offset, long length,
        Bool delete_value, Bool pending, Atom requested, Atom *type, int *format,
        unsigned long *count, unsigned long *remaining, unsigned char **bytes) {
    (void)d; (void)output; (void)property; (void)offset; (void)length; (void)delete_value; (void)pending; (void)requested;
    *type=XA_INTEGER; *format=8; *count=128; *remaining=0;
    *bytes=malloc(128); assert(*bytes); memcpy(*bytes,identity,128);
    if (fault==3 || changed(1)) (*bytes)[10]^=1;
    return Success;
}

int __wrap_av_hwdevice_ctx_create(AVBufferRef **output, enum AVHWDeviceType type, const char *device, AVDictionary *options, int flags) {
    (void)options; (void)flags;
    assert(type==AV_HWDEVICE_TYPE_VAAPI && !strncmp(device,"/proc/self/fd/",14));
    *output=av_buffer_allocz(sizeof(AVHWDeviceContext)+sizeof(AVVAAPIDeviceContext)); assert(*output);
    AVHWDeviceContext *context=(void *)(*output)->data;
    context->hwctx=(*output)->data+sizeof(*context);
    ((AVVAAPIDeviceContext *)context->hwctx)->display=(VADisplay)(uintptr_t)1;
    return 0;
}
AVBufferRef *__wrap_av_hwframe_ctx_alloc(AVBufferRef *device) {
    assert(device); return av_buffer_allocz(sizeof(AVHWFramesContext));
}
int __wrap_av_hwframe_ctx_init(AVBufferRef *reference) {
    AVHWFramesContext *frames=(void *)reference->data;
    assert(frames->format==AV_PIX_FMT_VAAPI && frames->sw_format==AV_PIX_FMT_NV12);
    assert(frames->width==128 && frames->height==128 && frames->initial_pool_size==4); return 0;
}
int __wrap_av_hwframe_get_buffer(AVBufferRef *pool, AVFrame *frame, int flags) {
    (void)flags; assert(pool && frame); frame->data[3]=(void *)(uintptr_t)2; return 0;
}
int __wrap_avcodec_open2(AVCodecContext *context, const AVCodec *codec, AVDictionary **options) {
    (void)options; assert(codec && context->hw_frames_ctx); assert(context->max_b_frames==0); return 0;
}
int __wrap_avcodec_send_frame(AVCodecContext *context, const AVFrame *frame) {
    assert(context);
    if (frame) { assert(synchronized); submitted++; pending_packet=1; synchronized=0; }
    return 0;
}
int __wrap_avcodec_receive_packet(AVCodecContext *context, AVPacket *packet) {
    (void)context;
    if (!pending_packet) return AVERROR(EAGAIN);
    pending_packet=0; assert(av_new_packet(packet,4)==0); memset(packet->data,0x5a,4);
    packet->pts=packet->dts=100; packet->duration=1; return 0;
}

VAStatus __wrap_vaCreateSurfaces(VADisplay display, unsigned format, unsigned width, unsigned height,
        VASurfaceID *surfaces, unsigned count, VASurfaceAttrib *attributes, unsigned nattrs) {
    (void)display; assert(format==VA_RT_FORMAT_RGB32 && width==128 && height==128 && count==1 && nattrs==2);
    const VADRMPRIMESurfaceDescriptor *descriptor=attributes[1].value.value.p;
    assert(descriptor->num_objects==1 && descriptor->num_layers==1);
    assert(descriptor->objects[0].fd==exported_fd && descriptor->layers[0].pitch[0]==512);
    surfaces[0]=1; imports++; return 0;
}
VAStatus __wrap_vaCreateConfig(VADisplay d, VAProfile profile, VAEntrypoint entry, VAConfigAttrib *attributes, int count, VAConfigID *config) {
    (void)d; (void)attributes; assert(profile==VAProfileNone && entry==VAEntrypointVideoProc && !count); *config=1; return 0;
}
VAStatus __wrap_vaCreateContext(VADisplay d, VAConfigID config, int width, int height, int flags, VASurfaceID *targets, int count, VAContextID *context) {
    (void)d; (void)flags; (void)targets; assert(config==1 && width==128 && height==128 && !count); *context=1; return 0;
}
VAStatus __wrap_vaCreateBuffer(VADisplay d, VAContextID context, VABufferType type, unsigned size, unsigned count, void *data, VABufferID *buffer) {
    (void)d; assert(context==1 && type==VAProcPipelineParameterBufferType && size==sizeof(VAProcPipelineParameterBuffer) && count==1);
    const VAProcPipelineParameterBuffer *parameters=data;
    assert(parameters->surface==1 && parameters->input_color_properties.color_range==VA_SOURCE_RANGE_FULL);
    assert(parameters->output_color_properties.color_range==VA_SOURCE_RANGE_REDUCED);
    *buffer=1; conversions++; return 0;
}
VAStatus __wrap_vaBeginPicture(VADisplay d, VAContextID context, VASurfaceID surface) {
    (void)d; assert(context==1 && surface==2); pictures++; return 0;
}
VAStatus __wrap_vaRenderPicture(VADisplay d, VAContextID context, VABufferID *buffers, int count) {
    (void)d; assert(context==1 && count==1 && buffers[0]==1); return 0;
}
VAStatus __wrap_vaEndPicture(VADisplay d, VAContextID context) { (void)d; assert(context==1); return 0; }
VAStatus __wrap_vaSyncSurface(VADisplay d, VASurfaceID surface) { (void)d; assert(surface==2); synchronized=1; return 0; }
VAStatus __wrap_vaDestroyBuffer(VADisplay d, VABufferID buffer)
{
    (void)d;
    assert(buffer == 1);
    /* T1283: failed destruction leaves the native buffer owned by the process. */
    return fault == 11 ? VA_STATUS_ERROR_OPERATION_FAILED : VA_STATUS_SUCCESS;
}
VAStatus __wrap_vaDestroyContext(VADisplay d, VAContextID context) { (void)d; assert(context==1); return 0; }
VAStatus __wrap_vaDestroyConfig(VADisplay d, VAConfigID config) { (void)d; assert(config==1); return 0; }
VAStatus __wrap_vaDestroySurfaces(VADisplay d, VASurfaceID *surfaces, int count) { (void)d; assert(count==1 && surfaces[0]==1); return 0; }

static void stream_case(const char *kind, int damage) {
    char *args[]={"gpu","screen","/dev/null","128","128","1","240","18","60000","2",(char *)kind};
    if (damage) setenv("BLENT_GPU_CADENCE","damage",1);
    else unsetenv("BLENT_GPU_CADENCE");
    setenv("BLENT_GPU_TRACE","1",1);
    int previous=submitted;
    assert(gpu_helper_main(11,args)==0);
    assert(submitted==previous+2 && conversions==pictures && imports>0);
    assert(fcntl(exported_fd,F_GETFD)==-1 && fcntl(producer_fd,F_GETFD)==-1);
}

static void identity_case(void) {
    char path[]="/tmp/blent-t766-edid-XXXXXX";
    int fd=mkstemp(path); assert(fd>=0); identity[127]=6;
    assert(write(fd,identity,128)==128); close(fd);
    gpu_capture capture={0}; gpu_options options={.connector=path};
    gpu_identity_load(&capture,&options); assert(capture.edid_length==128);
    capture.display=XOpenDisplay(NULL); assert(capture.display);
    XInternAtom(capture.display,"EDID",False);
    assert(gpu_output_matches(&capture,&options,1,"screen"));
    fault=3; assert(!gpu_output_matches(&capture,&options,1,"screen")); fault=0;
    XCloseDisplay(capture.display); unlink(path);
}

static void rejected_export(int which) {
    pid_t child=fork(); assert(child>=0);
    if (!child) {
        fault=which;
        char *args[]={"gpu","screen","/dev/null","128","128","1","240","18","60000","1","desktop"};
        gpu_helper_main(11,args); _exit(0);
    }
    int status; assert(waitpid(child,&status,0)==child);
    assert(WIFEXITED(status) && WEXITSTATUS(status)==2);
}

/* T773: full checks must reject transitions even with unchanged dimensions,
 * after a prior successful capture and without useful RandR events. */
static void transition_child(int kind, int post, int backlog) {
    gpu_capture c={0};
    gpu_options options={.connector="screen",.device="/dev/null",.width=128,.height=128,.scale=1};
    gpu_capture_open(&c,&options);
    memcpy(c.edid,identity,128); c.edid_length=128;
    XInternAtom(c.display,"EDID",False);
    gpu_capture_take(&c,&options,0); gpu_capture_release(&c);
    /* More queued events than the damage consumer's per-pass budget. */
    for (int i=0;i<backlog;i++) {
        XEvent event={.type=ClientMessage}; event.xclient.display=c.display;
        XPutBackEvent(c.display,&event);
    }
    copied=0; transition=kind; after_copy=post;
    gpu_capture_take(&c,&options,1);
    gpu_capture_release(&c); gpu_capture_close(&c);
}

static void transition_case(int kind, int post, int backlog) {
    pid_t child=fork(); assert(child>=0);
    if (!child) { transition_child(kind,post,backlog); _exit(0); }
    int status; assert(waitpid(child,&status,0)==child);
    assert(WIFEXITED(status) && WEXITSTATUS(status)==2);
}

static void t773_transitions(void) {
    for (int kind=1;kind<=10;kind++) {
        for (int post=0;post<2;post++) {
            transition_case(kind,post,0);
            transition_case(kind,post,513);
        }
    }
    /* Repeated fresh connections cannot inherit old capture metadata. */
    for (int i=0;i<3;i++) stream_case("desktop",0);
}

int main(void)
{
    identity_case();
    t773_transitions();
    stream_case("pattern",0); stream_case("desktop",0); stream_case("desktop",1);
    rejected_export(11); /* T1283: retirement failure must terminate admission. */
    rejected_export(1); rejected_export(2);
    for (int error=4; error<=10; error++) rejected_export(error);
    return 0;
}
