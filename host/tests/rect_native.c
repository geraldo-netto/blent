/* T767: actual JNI wrapper source, fake codec/EGL boundaries, real allocations. */
#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <limits.h>
#include "../../scripts/benchmarks/android-rect/native/rect_decode.c"
#include "../../scripts/benchmarks/android-rect/native/rect_hash.c"

struct ZSTD_DCtx_s { int alive; };
typedef struct { void *data; jlong capacity; } direct_buffer;
static int codecs_called, extensions;

ZSTD_DCtx *ZSTD_createDCtx(void) { ZSTD_DCtx *c=malloc(sizeof(*c)); assert(c); c->alive=1; return c; }
size_t ZSTD_freeDCtx(ZSTD_DCtx *c) { free(c); return 0; }
int LZ4_decompress_safe(const char *source, char *target, int bytes, int expected) {
    assert(bytes>0 && expected>0); codecs_called++;
    memset(target,(unsigned char)source[0],(size_t)expected); return expected;
}
size_t ZSTD_decompressDCtx(ZSTD_DCtx *c, void *target, size_t expected, const void *source, size_t bytes) {
    assert(c->alive && bytes>0 && expected>0); codecs_called++;
    memset(target,*(const unsigned char *)source,expected); return expected;
}
static jlong capacity(JNIEnv *env, jobject object) {
    (void)env; return object ? ((direct_buffer *)object)->capacity : -1;
}
static void *address(JNIEnv *env, jobject object) {
    (void)env; return object ? ((direct_buffer *)object)->data : NULL;
}
static const struct JNINativeInterface_ interface = {.GetDirectBufferCapacity=capacity, .GetDirectBufferAddress=address};

static EGLBoolean valid_surface(EGLDisplay display, EGLSurface surface) {
    return display==(EGLDisplay)(intptr_t)1 && surface==(EGLSurface)(intptr_t)2;
}
static EGLBoolean supported(EGLDisplay d, EGLSurface s, EGLint field) {
    assert(field==EGL_DISPLAY_PRESENT_TIME_ANDROID); return valid_surface(d,s);
}
static EGLBoolean next_frame(EGLDisplay d, EGLSurface s, EGLuint64KHR *id) {
    *id=77; return valid_surface(d,s);
}
static EGLBoolean presented(EGLDisplay d, EGLSurface s, EGLuint64KHR id, EGLint count, const EGLint *fields, EGLnsecsANDROID *values) {
    assert(count==1 && fields[0]==EGL_DISPLAY_PRESENT_TIME_ANDROID); *values=99;
    return valid_surface(d,s) && id==77;
}
__eglMustCastToProperFunctionPointerType eglGetProcAddress(const char *name) {
    if (!extensions) return NULL;
    if (!strcmp(name,"eglGetFrameTimestampSupportedANDROID")) return (__eglMustCastToProperFunctionPointerType)supported;
    if (!strcmp(name,"eglGetNextFrameIdANDROID")) return (__eglMustCastToProperFunctionPointerType)next_frame;
    assert(!strcmp(name,"eglGetFrameTimestampsANDROID")); return (__eglMustCastToProperFunctionPointerType)presented;
}
EGLBoolean eglSurfaceAttrib(EGLDisplay d, EGLSurface s, EGLint attribute, EGLint value) {
    assert(attribute==EGL_TIMESTAMPS_ANDROID && value==EGL_TRUE); return valid_surface(d,s);
}

static int decode_arguments(int offset, int bytes, int expected, int codec) {
    int valid=offset>=0 && offset<=64 && bytes>0 && bytes<=64-offset && expected>0 && expected<=32;
    return valid && (codec==1 || codec==2);
}

static void decode_case(JNIEnv *env, jlong context, int offset, int bytes, int expected, int codec) {
    unsigned char source[64], target[32]; memset(source,0xa5,sizeof(source)); memset(target,0,sizeof(target));
    direct_buffer input={source,64}, output={target,32};
    int allowed=decode_arguments(offset,bytes,expected,codec);
    int calls=codecs_called;
    assert(Java_com_blent_benchmark_RectNative_decode(env,NULL,context,codec,(jobject)&input,offset,bytes,(jobject)&output,expected)==allowed);
    assert(codecs_called==calls+allowed);
    if (allowed) for (int i=0;i<expected;i++) assert(target[i]==0xa5);
}
static void decode_bounds(JNIEnv *env, jlong context) {
    const int edges[]={INT_MIN,-1,0,1,2,31,32,63,64,65,3072000,3072001,INT_MAX};
    for (unsigned i=0;i<sizeof(edges)/sizeof(edges[0]);i++) {
        decode_case(env,context,edges[i],1,1,1);
        decode_case(env,context,0,edges[i],1,2);
        decode_case(env,context,0,1,edges[i],1);
        decode_case(env,context,0,1,1,edges[i]);
    }
    unsigned seed=767;
    for (int i=0;i<8192;i++) {
        int values[4];
        for (int j=0;j<4;j++) { seed=seed*1664525u+1013904223u; values[j]=(int)(seed>>24)-32; }
        decode_case(env,context,values[0],values[1],values[2],values[3]%4);
    }
    assert(!Java_com_blent_benchmark_RectNative_decode(env,NULL,0,1,NULL,0,1,NULL,1));
    assert(!Java_com_blent_benchmark_RectNative_decode(env,NULL,context,1,NULL,0,1,NULL,1));
    direct_buffer empty={NULL,64};
    assert(!Java_com_blent_benchmark_RectNative_decode(env,NULL,context,1,(jobject)&empty,0,1,(jobject)&empty,1));
}
static void hash_bounds(JNIEnv *env) {
    unsigned char rgba[32]={1,2,3,255}; direct_buffer buffer={rgba,32};
    uint64_t expected=UINT64_C(14695981039346656037);
    for (unsigned i=0;i<3;i++) expected=(expected^rgba[i])*UINT64_C(1099511628211);
    assert((uint64_t)Java_com_blent_benchmark_RectNative_rgbHash(env,NULL,(jobject)&buffer,1)==expected);
    const int counts[]={INT_MIN,-1,0,9,1024001,INT_MAX};
    for (unsigned i=0;i<sizeof(counts)/sizeof(counts[0]);i++)
        assert(!Java_com_blent_benchmark_RectNative_rgbHash(env,NULL,(jobject)&buffer,counts[i]));
    buffer.data=NULL; assert(!Java_com_blent_benchmark_RectNative_rgbHash(env,NULL,(jobject)&buffer,1));
}
static void timestamp_bounds(JNIEnv *env) {
    for (extensions=0;extensions<=1;extensions++) {
        assert(Java_com_blent_benchmark_RectNative_enableTimestamps(env,NULL,1,2)==extensions);
        assert(Java_com_blent_benchmark_RectNative_nextFrame(env,NULL,1,2)==(extensions ? 77 : -3));
        assert(Java_com_blent_benchmark_RectNative_presented(env,NULL,1,2,77)==(extensions ? 99 : -3));
        assert(!Java_com_blent_benchmark_RectNative_enableTimestamps(env,NULL,INT64_MIN,INT64_MAX));
        assert(Java_com_blent_benchmark_RectNative_nextFrame(env,NULL,0,0)==-3);
        assert(Java_com_blent_benchmark_RectNative_presented(env,NULL,1,2,-1)==-3);
    }
}
int main(void) {
    JNIEnv env=&interface;
    jlong context=Java_com_blent_benchmark_RectNative_create(&env,NULL); assert(context);
    decode_bounds(&env,context); hash_bounds(&env); timestamp_bounds(&env);
    Java_com_blent_benchmark_RectNative_destroy(&env,NULL,context);
    Java_com_blent_benchmark_RectNative_destroy(&env,NULL,0);
    return 0;
}
