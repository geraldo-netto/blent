#define _POSIX_C_SOURCE 200809L
#include <assert.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pthread.h>
static int scenario,calls,writes,transfers,pending_packet;
static ssize_t fake_read(int fd,void *bytes,size_t size);
static ssize_t fake_write(int fd,const void *bytes,size_t size);
static int fake_thread(pthread_t *thread,const pthread_attr_t *attr,void *(*start)(void *),void *data);
#define read fake_read
#define write fake_write
#define pthread_create fake_thread
#define _Exit exit
#define main adapter_main
#include "stream.c"
#undef main
#undef _Exit
#undef pthread_create
#undef write
#undef read
void accessory_open(accessory_usb *usb,accessory_target target) { (void)target;usb->input=128;usb->output_packet=512; }
void accessory_transfer(accessory_usb *usb,int receive,unsigned char *bytes,int length) {
    (void)usb;assert(!receive);transfers++;
    if(scenario>=15) {assert(length==(scenario==18?513:512));pending_packet=scenario!=18;return;}
    assert(length==4 && !memcmp(bytes,"test",4));
}
int libusb_bulk_transfer(libusb_device_handle *handle,unsigned char ep,unsigned char *data,int length,int *count,unsigned int timeout) {
    (void)handle;
    if(scenario>=15 && length==0) {assert(ep==0 && timeout==5000);*count=scenario==17?1:0;pending_packet=0;return scenario==16?LIBUSB_ERROR_TIMEOUT:0;}
    assert(ep==128 && length==65536 && timeout==1000);calls++;
    if(scenario==1) { *count=0;return LIBUSB_ERROR_TIMEOUT; }
    if(scenario==2) { *count=2;memcpy(data,"ok",2);return LIBUSB_ERROR_TIMEOUT; }
    if(scenario==3) return LIBUSB_ERROR_NO_DEVICE;
    if(scenario==4) { *count=-1;return 0; }
    if(scenario==5) { *count=65537;return 0; }
    if(scenario==6) { *count=0;return 0; }
    if(scenario==10 && calls>1) return LIBUSB_ERROR_NO_DEVICE;
    *count=3;memcpy(data,"usb",3);return 0;
}
static ssize_t fake_read(int fd,void *bytes,size_t size) {
    assert(fd==STDIN_FILENO && size==65536);calls++;
    if(scenario>=15) {int length=scenario==18?513:512;memset(bytes,0,(size_t)length);return calls==1?length:0;}
    if(scenario==8) {errno=EIO;return -1;}
    if(calls==1) {errno=EINTR;return -1;}
    if(calls==2) {memcpy(bytes,"test",4);return 4;}
    return 0;
}
static ssize_t fake_write(int fd,const void *bytes,size_t size) {
    assert(fd==STDOUT_FILENO);writes++;
    if(scenario==9) return 0;
    if(writes==1) {errno=EINTR;return -1;}
    if(scenario==10) {assert(size==3 && !memcmp(bytes,"usb",3));return 3;}
    assert(size==(writes==2?4:2));assert(!memcmp(bytes,writes==2?"test":"st",2));return 2;
}
static int fake_thread(pthread_t *thread,const pthread_attr_t *attr,void *(*start)(void *),void *data) {
    (void)thread;(void)attr;(void)start;(void)data;return scenario==12?EAGAIN:0;
}
int main(int argc,char **argv) {
    assert(argc==2);scenario=atoi(argv[1]);accessory_usb usb={.input=128,.output_packet=512};
    unsigned char bytes[65536];int empty=0;
    if(scenario<=6) {
        for(int i=0;i<(scenario==6?9:1);i++) {
            int count=read_usb(&usb,bytes,&empty);
            assert(count==(scenario==1?0:scenario==2?2:scenario==6?0:3));
        }
    } else if(scenario>=15) {
        if(scenario==19) usb.output_packet=64;
        if(scenario>=20) usb.output_packet=(uint16_t[]){0,7,1025,9}[scenario-20];
        send_bytes(&usb);assert(!pending_packet);
    }
    else if(scenario==7 || scenario==8) {send_bytes(&usb);assert(transfers==1);}
    else if(scenario==9) output_all(STDOUT_FILENO,(unsigned char *)"test",4);
    else if(scenario==10) receive_bytes(&usb);
    else if(scenario==11) {output_all(STDOUT_FILENO,(unsigned char *)"test",4);assert(writes==3);}
    else {
        char *arguments[]={"fixture","254","255",NULL};
        return adapter_main(scenario==14?1:3,arguments);
    }
    return 0;
}
