#define _GNU_SOURCE
#include "accessory.h"
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/file.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
#include <zlib.h>

static accessory_usb usb;
static void cleanup(void) { accessory_close(&usb); }
static void put32(unsigned char *bytes,uint32_t value) {
    for (int index=0;index<4;index++) bytes[index]=(unsigned char)(value>>(24-index*8));
}
static void put64(unsigned char *bytes,uint64_t value) {
    for (int index=0;index<8;index++) bytes[index]=(unsigned char)(value>>(56-index*8));
}
static void frame(FILE *output,uint64_t session,int sequence) {
    static const int sizes[]={1,64,512,4096,16384,65536};
    unsigned char payload[65536],header[24],reply[24]; int length=sizes[sequence%6];
    for (int index=0;index<length;index++) payload[index]=(unsigned char)(index*31+sequence*17);
    put32(header,0x42414f31); put64(header+4,session); put32(header+12,(uint32_t)sequence);
    put32(header+16,(uint32_t)length); put32(header+20,(uint32_t)crc32(0,payload,(uInt)length));
    uint64_t start=accessory_clock();
    accessory_transfer(&usb,0,header,sizeof(header)); accessory_transfer(&usb,0,payload,length);
    accessory_transfer(&usb,1,reply,sizeof(reply)); accessory_require(!memcmp(header,reply,sizeof(header)),"echo header mismatch");
    accessory_transfer(&usb,1,payload,length); uint64_t elapsed=accessory_clock()-start;
    for (int index=0;index<length;index++) accessory_require(payload[index]==(unsigned char)(index*31+sequence*17),"echo pixel pattern mismatch");
    accessory_require(fprintf(output,"%d,%d,%"PRIu64"\n",sequence,length,elapsed)>0,"result write");
}
static void exchange(accessory_target target,const char *path) {
    alarm(70); accessory_require(atexit(cleanup)==0,"cleanup registration");
    FILE *output=fopen(path,"wx"); accessory_require(output!=NULL,"result file already exists or cannot be created");
    accessory_open(&usb,target);
    fprintf(stderr,"Starting in 10 seconds; approve accessory access and tap Start USB test if needed.\n");
    struct timespec ready={.tv_sec=10}; nanosleep(&ready,NULL);
    fputs("sequence,payload_bytes,echo_roundtrip_ns\n",output);
    uint64_t session=accessory_clock()|1;
    for (int sequence=0;sequence<4096;sequence++) frame(output,session,sequence);
    accessory_require(fclose(output)==0,"result close");
    fprintf(stderr,"PASS: 4096 patterned frames. Closing does not restore normal USB mode; unplug/replug.\n");
}
static int wait_child(pid_t pid,int seconds) {
    uint64_t deadline=accessory_clock()+(uint64_t)seconds*1000000000;
    struct timespec pause={.tv_nsec=100000000};
    while (accessory_clock()<deadline) {
        int status; pid_t result=waitpid(pid,&status,WNOHANG);
        if (result==pid) return WIFEXITED(status)?WEXITSTATUS(status):2;
        accessory_require(result==0 || errno==EINTR,"wait for probe worker");
        nanosleep(&pause,NULL);
    }
    return -1;
}
int main(int argc,char **argv) {
    accessory_require(argc==4,"usage: aoa-probe BUS PORT.PATH NEW_RESULT.csv (changes target USB mode)");
    accessory_target target=accessory_parse_target(argv[1],argv[2]); int lock=accessory_lock(target);
    pid_t pid=fork(); accessory_require(pid>=0,"fork probe worker");
    if (!pid) { exchange(target,argv[3]); exit(0); }
    close(lock); int result=wait_child(pid,75);
    if (result>=0) return result;
    kill(pid,SIGKILL); result=wait_child(pid,2);
    accessory_require(result>=0,"native worker did not retire; port remains quarantined by its lock"); return 2;
}
