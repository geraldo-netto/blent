/* T846: run the real USB adapter against deterministic re-enumeration failures. */
#define _POSIX_C_SOURCE 200809L
#include "accessory.h"
#include <libusb.h>
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
struct libusb_device { int unused; };
struct libusb_context { int unused; };
struct libusb_device_handle { int unused; };
static struct libusb_device device;
static struct libusb_context context;
static struct libusb_device_handle handle;
static int switched, attempts, sleeps, error, persistent, strict, absent;
static int selection, controls;
static struct libusb_endpoint_descriptor endpoints[]={{2,0x81,512},{2,0x02,512}};
static struct libusb_interface_descriptor descriptor={255,255,0,2,0,0,endpoints};
static struct libusb_interface interface={&descriptor,1};
static struct libusb_config_descriptor config={1,&interface};
int libusb_init(libusb_context **out) { *out=&context; return 0; }
void libusb_exit(libusb_context *value) { assert(value==&context); }
uint8_t libusb_get_bus_number(libusb_device *value) { assert(value==&device); return 3; }
uint8_t libusb_get_device_address(libusb_device *value) { assert(value==&device); return switched?8:7; }
int libusb_get_port_numbers(libusb_device *value,uint8_t *ports,int count) {
    assert(value==&device && count>=1); ports[0]=2; return 1;
}
ssize_t libusb_get_device_list(libusb_context *value,libusb_device ***out) {
    static libusb_device *list[]={&device,NULL};
    assert(value==&context); *out=list;
    if (switched && absent && sleeps==1) return 0;
    return 1;
}
void libusb_free_device_list(libusb_device **list,int unref) { assert(list && unref==1); }
int libusb_open(libusb_device *value,libusb_device_handle **out) {
    assert(value==&device); attempts++;
    if (strict || (switched && (persistent || attempts==2))) return error;
    *out=&handle; return 0;
}
void libusb_close(libusb_device_handle *value) { assert(value==&handle); }
libusb_device *libusb_get_device(libusb_device_handle *value) { assert(value==&handle); return &device; }
int libusb_get_device_descriptor(libusb_device *value,struct libusb_device_descriptor *out) {
    assert(value==&device); out->idVendor=0x18d1; out->idProduct=switched?0x2d00:0x4ee8; return 0;
}
static void check_metadata(uint8_t type,uint16_t index,const unsigned char *data,uint16_t length) {
        const char *expected[]={"Blent",selection?"Blent":"Blent USB Probe",
            selection?"Blent USB connection":"T846 synthetic USB integrity test","1",
            "https://github.com/geraldo-netto/blent",selection?"blent-1":"blent-probe-1"};
        assert(type==0x40 && index<6 && length==strlen(expected[index])+1);
        assert(!memcmp(data,expected[index],length));
        fprintf(stderr,"metadata[%u]=%s\n",index,data);
}
int libusb_control_transfer(libusb_device_handle *value,uint8_t type,uint8_t request,
    uint16_t val,uint16_t index,unsigned char *data,uint16_t length,unsigned int timeout) {
    assert(value==&handle && val==0 && timeout==1000);
    (void)type; (void)index; controls++;
    if (request==51) { assert(length==2); data[0]=2; data[1]=0; }
    if (request==52) check_metadata(type,index,data,length);
    if (request==53) switched=1;
    return length;
}
int libusb_get_active_config_descriptor(libusb_device *value,struct libusb_config_descriptor **out) {
    assert(value==&device); *out=&config; return 0;
}
void libusb_free_config_descriptor(struct libusb_config_descriptor *value) { assert(value==&config); }
int libusb_claim_interface(libusb_device_handle *value,int number) { assert(value==&handle && number==0); return 0; }
int libusb_release_interface(libusb_device_handle *value,int number) { assert(value==&handle && number==0); return 0; }
int libusb_bulk_transfer(libusb_device_handle *value,unsigned char endpoint,
    unsigned char *data,int length,int *transferred,unsigned int timeout) {
    (void)value; (void)endpoint; (void)data; (void)length; (void)transferred; (void)timeout;
    abort();
}
int nanosleep(const struct timespec *duration,struct timespec *remaining) {
    (void)remaining; assert(duration->tv_sec==0 && duration->tv_nsec==100000000);
    assert(++sleeps<=100); return 0;
}
static void report(void) { fprintf(stderr,"attempts=%d sleeps=%d controls=%d\n",attempts,sleeps,controls); }
int main(int argc,char **argv) {
    assert(argc==5 || argc==6); error=atoi(argv[1]); persistent=atoi(argv[2]); strict=atoi(argv[3]); absent=atoi(argv[4]);
    if(argc==6) selection=atoi(argv[5]);
    assert(!atexit(report));
    accessory_usb usb={0}; accessory_target target={.bus=3,.ports={2},.count=1};
    if(selection) {
        accessory_selection selected={.address=7,.vendor=0x18d1,.product=0x4ee8};
        if(selection==1) selected.address=6;
        if(selection==2) selected.vendor=0x1234;
        if(selection==3) selected.product=0x1234;
        accessory_open_selected(&usb,target,selected);
    } else accessory_open(&usb,target);
    assert(usb.claimed); accessory_close(&usb); return 0;
}
