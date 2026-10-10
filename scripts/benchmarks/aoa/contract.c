#define _POSIX_C_SOURCE 200809L
#include "usb_probe.h"
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

void probe_require(int condition, const char *message) {
    if (!condition) { fprintf(stderr, "USB probe failed: %s\n", message); exit(2); }
}
uint64_t probe_clock(void) {
    struct timespec time;
    probe_require(clock_gettime(CLOCK_MONOTONIC, &time)==0, "monotonic clock");
    return (uint64_t)time.tv_sec*1000000000 + (uint64_t)time.tv_nsec;
}
static uint8_t number(const char *text, char **end) {
    probe_require(*text>='0' && *text<='9', "numeric bus/port required");
    errno=0; unsigned long value=strtoul(text,end,10);
    probe_require(!errno && value>0 && value<=255, "bus/port outside 1..255");
    return (uint8_t)value;
}
probe_target probe_parse_target(const char *bus, const char *ports) {
    probe_target result={0}; char *end;
    result.bus=number(bus,&end); probe_require(!*end,"invalid bus suffix");
    do {
        probe_require(result.count<7,"port path too deep");
        result.ports[result.count++]=number(ports,&end);
        if (!*end) break;
        probe_require(*end=='.',"invalid port separator"); ports=end+1;
    } while (1);
    return result;
}
