/* T749: guest-only diagnostic. Mapping may change CPU visibility; this is NOT
 * passive observation or a production workaround for third-party exporters. */
#define _GNU_SOURCE
#include "pattern.h"
#include <gbm.h>
#include <dlfcn.h>
#include <unistd.h>

static void readback(struct gbm_bo *bo,const char *mode) {
    unsigned height=!strcmp(mode,"strip") ? 16 : SH;
    uint32_t stride=0;void *map=NULL;
    unsigned char *pixels=gbm_bo_map(bo,0,0,SW,height,GBM_BO_TRANSFER_READ,&stride,&map);
    assert(pixels && stride>=SW*4);
    int frame=decode(pixels,stride,4),bad=-2;
    if(height==SH)bad=check_image(pixels,stride,4,&frame);
    fprintf(stderr,"T749_GBM %lld mode=%s frame=%d bad=%d stride=%u\n",stamp_us(),mode,frame,bad,stride);
    gbm_bo_unmap(bo,map);
}

struct gbm_bo *gbm_surface_lock_front_buffer(struct gbm_surface *surface) {
    guest_only();
    const char *mode=getenv("SNAP_READBACK");
    assert(mode && (!strcmp(mode,"full") || !strcmp(mode,"strip") || !strcmp(mode,"wait")));
    struct gbm_bo *(*real)(struct gbm_surface *)=dlsym(RTLD_NEXT,"gbm_surface_lock_front_buffer");assert(real);
    struct gbm_bo *bo=real(surface);assert(bo);
    unsigned width=gbm_bo_get_width(bo),height=gbm_bo_get_height(bo),format=gbm_bo_get_format(bo);
    fprintf(stderr,"T749_BO width=%u height=%u format=%.4s\n",width,height,(char *)&format);
    if(width==SW && height==SH) {
        /* Fixed pixel oracle is valid only for four-byte little-endian XRGB. */
        assert(format==GBM_FORMAT_XRGB8888);
        if(!strcmp(mode,"wait"))usleep(2000);
        else readback(bo,mode);
    }
    return bo;
}
