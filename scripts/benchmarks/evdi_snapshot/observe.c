#define _GNU_SOURCE
#include "pattern.h"
#include "../../../host/evdi/evdi_lib.h"
#include <dlfcn.h>
static struct evdi_buffer registered;
void evdi_register_buffer(evdi_handle handle,struct evdi_buffer buffer) {
    guest_only();registered=buffer;
    void (*real)(evdi_handle,struct evdi_buffer)=dlsym(RTLD_NEXT,"evdi_register_buffer");assert(real);real(handle,buffer);
}
static void sample_pixels(void) {
    const unsigned char *pixels=registered.buffer;
    const int rows[]={0,8,16,32,200,460,472};
    for(unsigned y=0;y<sizeof(rows)/sizeof(rows[0]);y++) {
        fprintf(stderr,"SNAP_SAMPLE row=%d",rows[y]);
        for(int x=0;x<40;x++)fprintf(stderr," %02x",pixels[(size_t)rows[y]*registered.stride+(x*16+8)*4]);
        fprintf(stderr,"\n");
    }
}
void evdi_grab_pixels(evdi_handle handle,struct evdi_rect *rects,int *count) {
    void (*real)(evdi_handle,struct evdi_rect *,int *)=dlsym(RTLD_NEXT,"evdi_grab_pixels");assert(real);
    real(handle,rects,count);
    if (*count>0 && registered.width==SW && registered.height==SH) {
        int frame,bad=check_image(registered.buffer,registered.stride,4,&frame);
        static int grabs;
        if(++grabs==200)sample_pixels();
        fprintf(stderr,"SNAP_RAW %lld frame=%d bad=%d rects=%d\n",stamp_us(),frame,bad,*count);
    }
}
