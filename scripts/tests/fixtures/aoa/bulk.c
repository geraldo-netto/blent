/* T846: valid USB ZLPs must not terminate a byte-stream read. */
#define main enumeration_main
#define libusb_bulk_transfer unused_bulk_transfer
#include "enumeration.c"
#undef libusb_bulk_transfer
#undef main
static int mode,calls;
int clock_gettime(clockid_t clock,struct timespec *value) {
    static int reads;
    assert(clock==CLOCK_MONOTONIC);
    value->tv_sec=mode==7 && reads++>0?6:1; value->tv_nsec=0; return 0;
}
int libusb_bulk_transfer(libusb_device_handle *value,unsigned char endpoint,
    unsigned char *data,int length,int *transferred,unsigned int timeout) {
    (void)value; (void)endpoint; (void)data;
    assert(timeout>0 && timeout<=5000); assert(++calls<=9);
    *transferred=0;
    if (mode==1) {
        if (calls%2==0) *transferred=length>2?2:length;
        return 0;
    }
    if (mode==4) return -7;
    if (mode==5) *transferred=length+1;
    if (mode==8) *transferred=-1;
    return 0;
}
static void report_bulk(void) { fprintf(stderr,"calls=%d\n",calls); }
int main(int argc,char **argv) {
    assert(argc==2); mode=atoi(argv[1]); assert(!atexit(report_bulk));
    probe_usb usb={0}; unsigned char data[4]={0};
    probe_transfer(&usb,mode!=3,data,mode==6?0:4); return 0;
}
