#define _POSIX_C_SOURCE 200809L
#include "accessory.h"
#include <libusb.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static int matches(libusb_device *device, accessory_target target) {
    uint8_t path[7];
    if (libusb_get_bus_number(device)!=target.bus) return 0;
    int length=libusb_get_port_numbers(device,path,sizeof(path));
    return length==target.count && memcmp(path,target.ports,(size_t)target.count)==0;
}
static libusb_device_handle *find_device(accessory_usb *usb, accessory_target target, int *open_error) {
    libusb_device **list=NULL; libusb_device_handle *handle=NULL;
    int found=0; *open_error=0;
    ssize_t count=libusb_get_device_list(usb->context,&list);
    accessory_require(count>=0 && count<=512,"device inventory unavailable or too large");
    for (ssize_t index=0;index<count;index++) {
        if (!matches(list[index],target)) continue;
        accessory_require(!found,"ambiguous physical USB path"); found=1;
        *open_error=libusb_open(list[index],&handle);
    }
    libusb_free_device_list(list,1); return handle;
}
static int is_accessory(libusb_device_handle *handle) {
    struct libusb_device_descriptor desc;
    accessory_require(libusb_get_device_descriptor(libusb_get_device(handle),&desc)==0,"device descriptor");
    return desc.idVendor==0x18d1 && (desc.idProduct==0x2d00 || desc.idProduct==0x2d01);
}
static void check_reopen(int status) {
    int retryable=status==0 || status==LIBUSB_ERROR_ACCESS || status==LIBUSB_ERROR_NO_DEVICE;
    if (!retryable) fprintf(stderr,"USB re-enumeration open error %d\n",status);
    accessory_require(retryable,"cannot open re-enumerating target");
}
static void start_accessory(libusb_device_handle *handle, int production) {
    unsigned char version[2];
    int count=libusb_control_transfer(handle,0xc0,51,0,0,version,2,1000);
    accessory_require(count==2 && (version[0] || version[1]),"AOA unsupported");
    const char *strings[]={"Blent",production?"Blent":"Blent USB Probe",
                          production?"Blent USB connection":"T846 synthetic USB integrity test","1",
                          "https://github.com/geraldo-netto/blent",production?"blent-1":"blent-probe-1"};
    for (int index=0;index<6;index++) {
        int length=(int)strlen(strings[index])+1;
        count=libusb_control_transfer(handle,0x40,52,0,(uint16_t)index,
                                     (unsigned char *)strings[index],(uint16_t)length,1000);
        accessory_require(count==length,"AOA identification transfer");
    }
    accessory_require(libusb_control_transfer(handle,0x40,53,0,0,NULL,0,1000)==0,"AOA mode switch");
}
static void wait_accessory(accessory_usb *usb, accessory_target target) {
    struct timespec pause={.tv_nsec=100000000};
    int last_error=0;
    for (int attempt=0;attempt<100;attempt++) {
        int open_error=0;
        nanosleep(&pause,NULL); usb->handle=find_device(usb,target,&open_error);
        check_reopen(open_error);
        if (open_error) last_error=open_error;
        if (!usb->handle) continue;
        if (is_accessory(usb->handle)) return;
        libusb_close(usb->handle); usb->handle=NULL;
    }
    if (last_error) fprintf(stderr,"Last USB open error %d during re-enumeration.\n",last_error);
    accessory_require(0,"accessory did not enumerate at the selected physical port");
}
static int accessory_interface(const struct libusb_interface_descriptor *desc) {
    return desc->bInterfaceClass==0xff && desc->bInterfaceSubClass==0xff &&
           desc->bInterfaceProtocol==0 && desc->bNumEndpoints==2 && desc->bAlternateSetting==0;
}
static int endpoints(const struct libusb_interface_descriptor *desc, accessory_usb *usb) {
    if (!accessory_interface(desc)) return 0;
    unsigned char input=0,output=0; uint16_t packet=0;
    for (int index=0;index<2;index++) {
        const struct libusb_endpoint_descriptor *ep=&desc->endpoint[index];
        if ((ep->bmAttributes & LIBUSB_TRANSFER_TYPE_MASK)!=LIBUSB_TRANSFER_TYPE_BULK) return 0;
        if (ep->bEndpointAddress & LIBUSB_ENDPOINT_IN) input=ep->bEndpointAddress;
        else { output=ep->bEndpointAddress; packet=ep->wMaxPacketSize; }
    }
    if (!input || !output) return 0;
    accessory_require(packet>=8 && packet<=1024 && !(packet & (packet-1)),"invalid bulk output packet size");
    accessory_require(usb->interface<0,"ambiguous accessory interface");
    usb->interface=desc->bInterfaceNumber; usb->input=input; usb->output=output; usb->output_packet=packet; return 1;
}
static void claim_accessory(accessory_usb *usb) {
    struct libusb_config_descriptor *config=NULL;
    accessory_require(libusb_get_active_config_descriptor(libusb_get_device(usb->handle),&config)==0,"active USB configuration");
    for (int index=0;index<config->bNumInterfaces;index++) {
        const struct libusb_interface *item=&config->interface[index];
        for (int alt=0;alt<item->num_altsetting;alt++) endpoints(&item->altsetting[alt],usb);
    }
    libusb_free_config_descriptor(config);
    accessory_require(usb->interface>=0,"accessory bulk interface unavailable");
    accessory_require(libusb_claim_interface(usb->handle,usb->interface)==0,"accessory interface busy or unavailable");
    usb->claimed=1;
}
static void check_selection(libusb_device_handle *handle,const accessory_selection *selected) {
    if(!selected) return;
    libusb_device *device=libusb_get_device(handle);
    struct libusb_device_descriptor desc;
    accessory_require(libusb_get_device_descriptor(device,&desc)==0,"selected device descriptor");
    accessory_require(libusb_get_device_address(device)==selected->address &&
                      desc.idVendor==selected->vendor && desc.idProduct==selected->product,
                      "selected device changed; refresh inventory before retrying");
}
static void open_target(accessory_usb *usb, accessory_target target,const accessory_selection *selected) {
    int open_error=0;
    usb->interface=-1;
    accessory_require(libusb_init(&usb->context)==0,"libusb initialization");
    usb->handle=find_device(usb,target,&open_error);
    accessory_require(open_error==0,"cannot open target; check USB permission");
    accessory_require(usb->handle!=NULL,"target absent");
    check_selection(usb->handle,selected);
    if (!is_accessory(usb->handle)) {
        start_accessory(usb->handle,selected!=NULL); libusb_close(usb->handle); usb->handle=NULL;
        wait_accessory(usb,target);
    }
    claim_accessory(usb);
    struct libusb_device_descriptor desc;
    accessory_require(libusb_get_device_descriptor(libusb_get_device(usb->handle),&desc)==0,"accessory identity");
    fprintf(stderr,"Accessory %04x:%04x interface %d; approve %s on tablet.\n",desc.idVendor,desc.idProduct,usb->interface,
            selected?"Blent":"Blent USB Probe");
    fprintf(stderr,"ADB interface advertised: %s\n",desc.idProduct==0x2d01?"yes":"no");
}
void accessory_open(accessory_usb *usb,accessory_target target) { open_target(usb,target,NULL); }
void accessory_open_selected(accessory_usb *usb,accessory_target target,accessory_selection selected) {
    accessory_require(selected.address>0 && selected.address<=127,"invalid selected USB address");
    open_target(usb,target,&selected);
}
static void check_transfer(int status,int count,int remaining,int receive) {
    int valid=status==0 && count>=0 && count<=remaining;
    if (!valid) fprintf(stderr,"Bulk %s status=%d count=%d remaining=%d\n",
                        receive?"read":"write",status,count,remaining);
    accessory_require(valid,"bulk transfer failed, timed out or stalled");
}
static unsigned int transfer_timeout(uint64_t deadline) {
    uint64_t now=accessory_clock();
    accessory_require(now<deadline,"bulk transfer deadline expired");
    return (unsigned int)((deadline-now+999999)/1000000);
}
static void check_progress(int count,int receive,int *empty) {
    if (count) { *empty=0; return; }
    accessory_require(receive && ++*empty<=8,"bulk transfer made no progress");
}
void accessory_transfer(accessory_usb *usb,int receive,unsigned char *data,int length) {
    accessory_require(length>0 && length<=65536,"invalid transfer length");
    int offset=0,empty=0;
    uint64_t deadline=accessory_clock()+UINT64_C(5000000000);
    while (offset<length) {
        int count=0;
        int status=libusb_bulk_transfer(usb->handle,receive?usb->input:usb->output,
                                       data+offset,length-offset,&count,transfer_timeout(deadline));
        check_transfer(status,count,length-offset,receive);
        check_progress(count,receive,&empty);
        offset+=count;
    }
}
void accessory_close(accessory_usb *usb) {
    if (usb->handle) {
        if (usb->claimed) libusb_release_interface(usb->handle,usb->interface);
        libusb_close(usb->handle);
    }
    if (usb->context) libusb_exit(usb->context);
}
