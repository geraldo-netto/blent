#define _POSIX_C_SOURCE 200809L
#include "accessory.h"
#include <errno.h>
#include <fcntl.h>
#include <sys/file.h>
#include <sys/stat.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

void accessory_require(int condition, const char *message) {
    if (!condition) { fprintf(stderr, "USB probe failed: %s\n", message); exit(2); }
}
uint64_t accessory_clock(void) {
    struct timespec time;
    accessory_require(clock_gettime(CLOCK_MONOTONIC, &time)==0, "monotonic clock");
    return (uint64_t)time.tv_sec*1000000000 + (uint64_t)time.tv_nsec;
}
static uint8_t number(const char *text, char **end) {
    accessory_require(*text>='0' && *text<='9', "numeric bus/port required");
    errno=0; unsigned long value=strtoul(text,end,10);
    accessory_require(!errno && value>0 && value<=255, "bus/port outside 1..255");
    return (uint8_t)value;
}
accessory_target accessory_parse_target(const char *bus, const char *ports) {
    accessory_target result={0}; char *end;
    result.bus=number(bus,&end); accessory_require(!*end,"invalid bus suffix");
    do {
        accessory_require(result.count<7,"port path too deep");
        result.ports[result.count++]=number(ports,&end);
        if (!*end) break;
        accessory_require(*end=='.',"invalid port separator"); ports=end+1;
    } while (1);
    return result;
}

int accessory_lock(accessory_target target) {
    char name[160];
    int offset=snprintf(name,sizeof(name),"/run/user/%u/blent-aoa-probe-%u",(unsigned)getuid(),target.bus);
    for (int index=0;index<target.count;index++) offset+=snprintf(name+offset,sizeof(name)-(size_t)offset,"-%u",target.ports[index]);
    int fd=open(name,O_CREAT|O_RDWR|O_NOFOLLOW|O_CLOEXEC,0600); accessory_require(fd>=0,"cannot open probe ownership lock");
    struct stat status; accessory_require(fstat(fd,&status)==0,"probe lock metadata");
    accessory_require(S_ISREG(status.st_mode) && status.st_uid==getuid(),"unsafe probe ownership lock");
    accessory_require(flock(fd,LOCK_EX|LOCK_NB)==0,"another probe still owns this port; do not start a replacement"); return fd;
}
