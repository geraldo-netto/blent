#define _GNU_SOURCE
#include "pattern.h"
#include <fcntl.h>
#include <unistd.h>
#include <poll.h>
#include <errno.h>
static int startup_placeholder(const unsigned char *pixels) {
    for (int i=0;i<SSIZE;i++) if (pixels[i]!=(i<SW*SH ? luma(24) : 128)) return 0;
    return 1;
}
static int prelude(const unsigned char *pixels,int last) {
    if(last>=0)return 0;
    if(startup_placeholder(pixels))return 1;
    return getenv("SNAP_COMPOSITOR") && decode(pixels,SW,1)<0;
}
static int read_frame(int fd,unsigned char *pixels,size_t *offset) {
    struct pollfd pollfd={fd,POLLIN,0}; if (poll(&pollfd,1,100)<=0) return 0;
    ssize_t got=read(fd,pixels+*offset,SSIZE-*offset);
    if (got<=0) { usleep(1000);return 0; }
    *offset+=(size_t)got;return *offset==SSIZE;
}
int main(void) {
    guest_only();setbuf(stdout,NULL);
    unsigned char *pixels=malloc(SSIZE);assert(pixels);
    int fd=open("/tmp/video",O_RDONLY|O_NONBLOCK);assert(fd>=0);
    size_t offset=0;int frames=0,invalid=0,last=-1,backward=0,placeholders=0;
    long long end=stamp_us()+16000000;
    while (stamp_us()<end) {
        if (!read_frame(fd,pixels,&offset)) continue;
        if (prelude(pixels,last)) { placeholders++;offset=0;continue; }
        int frame,bad=check_image(pixels,SW,1,&frame);frames++;invalid+=bad!=0;
        if (frame<last) backward++;
        last=frame;offset=0;
        printf("SNAP_NV12 %lld frame=%d bad=%d\n",stamp_us(),frame,bad);
    }
    printf("SNAP_CONSUMER frames=%d invalid=%d backward=%d last=%d partial=%zu placeholders=%d\n",frames,invalid,backward,last,offset,placeholders);
    free(pixels);close(fd);return !frames || invalid || backward || last!=SN;
}
