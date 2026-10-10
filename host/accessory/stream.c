#define _POSIX_C_SOURCE 200809L
#include "accessory.h"
#include <libusb.h>
#include <errno.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

/* Opaque transport bytes only. Parent owns TLS/approval/deadlines. Process exit
 * retires both native directions; never close libusb under a running transfer. */
static void output_all(int fd,const unsigned char *bytes,int length) {
    int offset=0;
    while (offset<length) {
        ssize_t count=write(fd,bytes+offset,(size_t)(length-offset));
        if (count<0 && errno==EINTR) continue;
        accessory_require(count>0,"parent stream write failed");
        offset+=(int)count;
    }
}
static int read_usb(accessory_usb *usb,unsigned char *bytes,int *empty) {
    int count=0;
    int status=libusb_bulk_transfer(usb->handle,usb->input,bytes,65536,&count,1000);
    accessory_require(status==0 || status==LIBUSB_ERROR_TIMEOUT,"USB receive failed");
    accessory_require(count>=0 && count<=65536,"invalid USB receive extent");
    if (count>0 || status==LIBUSB_ERROR_TIMEOUT) { *empty=0; return count; }
    accessory_require(++*empty<=8,"USB receive made no progress");
    return 0;
}
static void *receive_bytes(void *argument) {
    accessory_usb *usb=argument;
    unsigned char bytes[65536]; int empty=0;
    for (;;) {
        int length=read_usb(usb,bytes,&empty);
        if (length) output_all(STDOUT_FILENO,bytes,length);
    }
    return NULL;
}
static void send_usb(accessory_usb *usb,unsigned char *bytes,int length) {
    accessory_require(usb->output_packet>=8 && usb->output_packet<=1024 && !(usb->output_packet & (usb->output_packet-1)),"invalid output packet capacity");
    accessory_transfer(usb,0,bytes,length);
    if (length % usb->output_packet) return;
    /* A full final USB packet does not finish a larger gadget read request.
     * Terminate this pipe chunk explicitly; its peer may await it before replying. */
    int count=-1;
    int status=libusb_bulk_transfer(usb->handle,usb->output,NULL,0,&count,5000);
    accessory_require(status==0 && count==0,"USB output terminator failed");
}
static void send_bytes(accessory_usb *usb) {
    unsigned char bytes[65536];
    for (;;) {
        ssize_t count=read(STDIN_FILENO,bytes,sizeof(bytes));
        if (count<0 && errno==EINTR) continue;
        accessory_require(count>=0,"parent stream read failed");
        if (!count) return;
        send_usb(usb,bytes,(int)count);
    }
}
int main(int argc,char **argv) {
    accessory_require(argc==3,"usage: blent-usb-io BUS PORT.PATH");
    accessory_target target=accessory_parse_target(argv[1],argv[2]);
    int lock=accessory_lock(target);
    accessory_usb usb={0};accessory_open(&usb,target);
    pthread_t reader;
    accessory_require(pthread_create(&reader,NULL,receive_bytes,&usb)==0,"USB reader unavailable");
    send_bytes(&usb);
    (void)lock;
    /* Exit all threads together; no libusb cleanup racing its reader. The
     * supervisor retains admission until waitpid confirms process retirement. */
    _Exit(0);
}
