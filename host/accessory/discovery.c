#define _POSIX_C_SOURCE 200809L
#include "accessory.h"
#include <libusb.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* T876 read-only inventory. Never open/claim a USB device or issue requests.
 * The parent bounds the complete helper lifetime independently of libusb. */
#define INVENTORY_MAX 512
#define INVENTORY_HEADER 16
#define INVENTORY_RECORD 16

static int inventory_limit(const char *text) {
    accessory_require(text && *text>='1' && *text<='9',"invalid discovery capacity");
    char *end=NULL; errno=0;
    unsigned long value=strtoul(text,&end,10);
    accessory_require(!errno && !*end && value>=1 && value<=INVENTORY_MAX,"discovery capacity outside 1..512");
    return (int)value;
}
static void put16(unsigned char *bytes,uint16_t value) {
    bytes[0]=(unsigned char)(value>>8); bytes[1]=(unsigned char)value;
}
static int inventory_record(libusb_device *device,unsigned char *bytes) {
    uint8_t ports[7]={0};
    int count=libusb_get_port_numbers(device,ports,sizeof(ports));
    accessory_require(count>=0 && count<=7,"invalid discovery port depth");
    if (!count) return 0; /* Root controller, not a selectable peripheral. */
    struct libusb_device_descriptor descriptor;
    accessory_require(libusb_get_device_descriptor(device,&descriptor)==0,"discovery descriptor unavailable");
    if (descriptor.bDeviceClass==9) return 0; /* USB hubs cannot be accessory targets. */
    bytes[0]=libusb_get_bus_number(device); bytes[1]=libusb_get_device_address(device);
    accessory_require(bytes[0]>0 && bytes[1]>0 && bytes[1]<=127,"invalid discovery bus/address");
    bytes[2]=(unsigned char)count; bytes[3]=descriptor.bDeviceClass;
    put16(bytes+4,descriptor.idVendor); put16(bytes+6,descriptor.idProduct);
    for (int index=0;index<count;index++) {
        accessory_require(ports[index]>0,"invalid discovery port"); bytes[8+index]=ports[index];
    }
    return 1;
}
static int inventory_records(libusb_context *context,int limit,unsigned char *bytes) {
    libusb_device **devices=NULL;
    ssize_t length=libusb_get_device_list(context,&devices);
    accessory_require(length>=0 && length<=INVENTORY_MAX,"device inventory unavailable or too large");
    int count=0;
    for (ssize_t index=0;index<length;index++) {
        unsigned char record[INVENTORY_RECORD]={0};
        if (!inventory_record(devices[index],record)) continue;
        accessory_require(count<limit,"discovery capacity exceeded");
        memcpy(bytes+INVENTORY_HEADER+count*INVENTORY_RECORD,record,sizeof(record)); count++;
    }
    libusb_free_device_list(devices,1);
    return count;
}
int accessory_list(const char *text) {
    int limit=inventory_limit(text);
    unsigned char bytes[INVENTORY_HEADER+INVENTORY_MAX*INVENTORY_RECORD]={0};
    memcpy(bytes,"BLUI\0\0\0\1",8);
    libusb_context *context=NULL;
    accessory_require(libusb_init(&context)==0,"discovery initialization failed");
    int count=inventory_records(context,limit,bytes);
    libusb_exit(context);
    put16(bytes+8,(uint16_t)count);
    size_t length=INVENTORY_HEADER+(size_t)count*INVENTORY_RECORD;
    accessory_require(fwrite(bytes,1,length,stdout)==length,"discovery output failed");
    accessory_require(fflush(stdout)==0,"discovery output flush failed");
    return 0;
}
