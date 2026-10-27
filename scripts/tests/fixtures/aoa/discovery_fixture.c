/* T876 deterministic inventory. Forbidden USB operations abort immediately. */
#include "accessory.h"
#include <libusb.h>
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
struct libusb_device { int index; };
struct libusb_context { int unused; };
static struct libusb_context context;
static struct libusb_device devices[513];
static libusb_device *entries[514];
static int mode, freed, exited;
int libusb_init(libusb_context **out) { *out=&context; return mode==4?-1:0; }
void libusb_exit(libusb_context *value) { assert(value==&context);exited++; }
ssize_t libusb_get_device_list(libusb_context *value,libusb_device ***out) {
    assert(value==&context); *out=entries;
    if (mode==1) return 0;
    if (mode==2) return 513;
    if (mode==3) return -1;
    return mode==12?512:5;
}
void libusb_free_device_list(libusb_device **list,int unref) { assert(list==entries && unref==1);freed++; }
uint8_t libusb_get_bus_number(libusb_device *value) { return mode==7?0:(uint8_t)(value->index/127+1); }
uint8_t libusb_get_device_address(libusb_device *value) {
    if (mode==8) return 0;
    if (mode==9) return 128;
    return (uint8_t)(value->index%127+1);
}
int libusb_get_port_numbers(libusb_device *value,uint8_t *ports,int capacity) {
    assert(capacity==7);
    if (mode==5) return -1;
    if (mode==6) return 8;
    memset(ports,0,7);
    ports[0]=mode==10?0:(uint8_t)(value->index%255+1);
    if (mode==12) return 1;
    if (value->index==0) return 0;
    if (value->index==2) { for (int i=0;i<7;i++) ports[i]=(uint8_t)(i+1);return 7; }
    return 1;
}
int libusb_get_device_descriptor(libusb_device *value,struct libusb_device_descriptor *out) {
    memset(out,0,sizeof(*out));
    if (mode==11) return -1;
    out->idVendor=0x18d1;out->idProduct=(uint16_t)(0x2d00+(value->index==4));
    if (mode==12) return 0;
    if (value->index==1) out->bDeviceClass=9;
    if (value->index==2) {out->idVendor=0x1234;out->idProduct=0x5678;out->bDeviceClass=255;}
    return 0;
}
int libusb_open(libusb_device *value,libusb_device_handle **out) { (void)value;(void)out;abort(); }
int libusb_claim_interface(libusb_device_handle *value,int interface) { (void)value;(void)interface;abort(); }
int libusb_control_transfer(libusb_device_handle *value,uint8_t type,uint8_t request,
    uint16_t val,uint16_t index,unsigned char *data,uint16_t length,unsigned int timeout) {
    (void)value;(void)type;(void)request;(void)val;(void)index;(void)data;(void)length;(void)timeout;abort();
}
static void report(void) { fprintf(stderr,"freed=%d exited=%d\n",freed,exited); }
int main(int argc,char **argv) {
    assert(argc==3);mode=atoi(argv[1]);assert(!atexit(report));
    for (int index=0;index<513;index++) {devices[index].index=index;entries[index]=&devices[index];}
    return accessory_list(argv[2]);
}
