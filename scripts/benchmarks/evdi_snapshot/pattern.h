#ifndef SNAPSHOT_PATTERN_H
#define SNAPSHOT_PATTERN_H
#ifdef NDEBUG
#error "Snapshot fixture requires assertions"
#endif
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <assert.h>
enum { SW=640, SH=480, SN=240, STILES=80, SSIZE=SW*SH*3/2 };
static long long stamp_us(void) { struct timespec t; assert(!clock_gettime(CLOCK_MONOTONIC,&t)); return (long long)t.tv_sec*1000000+t.tv_nsec/1000; }
static void guest_only(void) {
    char text[4096]={0}; FILE *file=fopen("/proc/cmdline","r");
    if (!file || !fgets(text,sizeof(text),file) || !strstr(text,"blent-evdi-review=1")) exit(64);
    fclose(file);
}
static unsigned gray(unsigned generation) { return 32 + generation*29%192; }
static unsigned luma(unsigned gray_value) { return (220*gray_value+128)/256+16; }
static unsigned expected(unsigned frame,int x,int y) {
    if (y<16) {
        int bit=x/16;
        if (bit>=24) return 128;
        unsigned id=bit<12 ? frame : frame^4095;
        return (id>>(bit%12)&1)*255;
    }
    if (y<32) return 89;
    unsigned tile=(unsigned)((y-32)/56*10+x/64);
    unsigned generation=frame>tile ? frame-(frame-1-tile)%STILES : 0;
    return gray(generation);
}
static int decode(const unsigned char *pixels,int stride,int channels) {
    unsigned id=0,complement=0;
    for (int bit=0;bit<12;bit++) {
        id|=(unsigned)(pixels[(bit*16+8)*channels]>127)<<bit;
        complement|=(unsigned)(pixels[((bit+12)*16+8)*channels]>127)<<bit;
    }
    (void)stride;
    return (id^complement)==4095 && id<=SN ? (int)id : -1;
}
static int check_row(const unsigned char *pixels,int stride,int channels,int frame,int y) {
    for (int x=0;x<SW;x++) {
        unsigned value=expected((unsigned)frame,x,y);
        if (channels==1) value=luma(value);
        for (int channel=0;channel<(channels==1 ? 1 : 3);channel++)
            if (pixels[(size_t)y*stride+x*channels+channel]!=value) return y*SW+x+1;
    }
    return 0;
}
static int check_image(const unsigned char *pixels,int stride,int channels,int *frame) {
    *frame=decode(pixels,stride,channels);
    if (*frame<0) return -1;
    for (int y=0;y<SH;y++) { int bad=check_row(pixels,stride,channels,*frame,y); if (bad) return bad; }
    if (channels==1) for (int i=SW*SH;i<SSIZE;i++) if (pixels[i]!=128) return SSIZE;
    return 0;
}
#endif
