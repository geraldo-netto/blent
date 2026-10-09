/* T773: real metadata round trips on a private Xvfb; no DRI3/VAAPI device. */
#define _POSIX_C_SOURCE 200809L
#include "checked_arguments.h"
#include "../../host/gpu/capture.c"
#include "../../host/gpu/identity.c"
#include <assert.h>
#include <inttypes.h>
#include <stdio.h>

enum { SAMPLES=256 };
static int compare(const void *a,const void *b) {
    uint64_t x=*(const uint64_t *)a,y=*(const uint64_t *)b;
    return (x>y)-(x<y);
}
static void init(gpu_capture *c,gpu_options *options,int identity) {
    c->display=XOpenDisplay(NULL);assert(c->display);c->root=DefaultRootWindow(c->display);
    XRRScreenResources *resources=XRRGetScreenResourcesCurrent(c->display,c->root);
    assert(resources && resources->noutput==1);c->output=resources->outputs[0];
    XRROutputInfo *output=XRRGetOutputInfo(c->display,resources,c->output);assert(output);
    options->connector=strdup(output->name);options->scale=1;c->crtc=output->crtc;
    XRRCrtcInfo *info=XRRGetCrtcInfo(c->display,resources,c->crtc);assert(info);
    c->width=options->width=info->width;c->height=options->height=info->height;c->x=info->x;c->y=info->y;
    if(identity) {
        c->edid_length=128;memset(c->edid,0x5a,128);
        Atom property=XInternAtom(c->display,"EDID",False);
        XRRChangeOutputProperty(c->display,c->output,property,XA_INTEGER,8,PropModeReplace,c->edid,128);
    }
    XRRFreeCrtcInfo(info);XRRFreeOutputInfo(output);XRRFreeScreenResources(resources);XSync(c->display,False);
}
static void measure(gpu_capture *c,const gpu_options *options,int identity) {
    uint64_t values[SAMPLES];unsigned long requests=0;
    for(int i=0;i<SAMPLES;i++) {
        unsigned long before=NextRequest(c->display);uint64_t start=gpu_now_ns();
        check_geometry(c,options);check_geometry(c,options);
        values[i]=gpu_now_ns()-start;requests+=NextRequest(c->display)-before;
    }
    qsort(values,SAMPLES,sizeof(*values),compare);
    /* Xlib caches successful atom lookups on this Display already. */
    unsigned long atom_before=NextRequest(c->display);
    for (int i=0;i<SAMPLES;i++) (void)XInternAtom(c->display,"EDID",True);
    unsigned long atom_requests=NextRequest(c->display)-atom_before;
    printf("{\"identity\":%d,\"samples\":%d,\"metadata_pair_p50_ns\":%"PRIu64",\"metadata_pair_p95_ns\":%"PRIu64",\"x_requests_per_pair\":%.3f,\"atom_requests_after_warmup\":%lu}\n",
        identity,SAMPLES,values[SAMPLES/2],values[SAMPLES*95/100],(double)requests/SAMPLES,atom_requests);
}
int main(int argc,char **argv) {
    const char *isolated=getenv("BLENT_C_REVIEW_ISOLATED");
    assert(isolated && !strcmp(isolated,"1"));
    assert(argc==2);int identity=benchmark_number(argv[1],0,1);
    gpu_capture capture={0};gpu_options options={0};init(&capture,&options,identity);
    measure(&capture,&options,identity);free((void *)options.connector);XCloseDisplay(capture.display);return 0;
}
