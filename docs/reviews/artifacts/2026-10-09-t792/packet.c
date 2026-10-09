#define _POSIX_C_SOURCE 200809L
#include "output.c"
#include <assert.h>
#include "checked_arguments.h"
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/uio.h>
#include <sys/wait.h>
#include <time.h>
#include <errno.h>
void gpu_require(int c,const char *operation) { if(!c) { perror(operation);exit(2); } }
static uint64_t stamp(clockid_t clock) {struct timespec t;assert(!clock_gettime(clock,&t));return (uint64_t)t.tv_sec*1000000000+t.tv_nsec;}
static void vector(const AVPacket *packet) {
 char header[128];unsigned checksum=av_adler32_update(0,packet->data,packet->size);
 int n=snprintf(header,sizeof(header),"0, %" PRId64 ", %" PRId64 ", %" PRId64 ", %d, 0x%08x\n",packet->dts,packet->pts,packet->duration,packet->size,checksum);assert(n>0 && n<(int)sizeof(header));
 struct iovec v[2]={{header,(size_t)n},{packet->data,(size_t)packet->size}};int first=0;
 while(first<2) { ssize_t count=writev(STDOUT_FILENO,v+first,2-first);if(count<0 && errno==EINTR)continue;assert(count>0);
  size_t used=count;while(first<2 && used>=v[first].iov_len){used-=v[first].iov_len;first++;}if(first<2){v[first].iov_base=(char*)v[first].iov_base+used;v[first].iov_len-=used;}
 }
}
static void reader(int fd,size_t expected) {char b[65536];size_t count=0;ssize_t n;while((n=read(fd,b,sizeof(b)))>0)count+=n;assert(!n && count==expected);close(fd);_exit(0);}
int main(int argc,char **argv) {
 assert(argc==3);int size=benchmark_number(argv[1],64,1048576),direct=benchmark_number(argv[2],0,1);
 unsigned char *bytes=malloc(size);assert(bytes);memset(bytes,63,size);AVPacket packet={.data=bytes,.size=size,.dts=123,.pts=123,.duration=1};
 char header[128];int h=snprintf(header,sizeof(header),"0, 123, 123, 1, %d, 0x%08x\n",size,(unsigned)av_adler32_update(0,bytes,size));int p[2];assert(!pipe(p));assert(!fflush(stdout));int saved=dup(1);assert(saved>=0);pid_t child=fork();assert(child>=0);
 enum{N=256};if(!child){close(p[1]);close(saved);reader(p[0],(size_t)(size+h)*N);}close(p[0]);assert(dup2(p[1],1)==1);close(p[1]);
 uint64_t wall=stamp(CLOCK_MONOTONIC),cpu=stamp(CLOCK_PROCESS_CPUTIME_ID);
 for(int i=0;i<N;i++){if(direct)vector(&packet);else gpu_emit_packet(&packet);}
 cpu=stamp(CLOCK_PROCESS_CPUTIME_ID)-cpu;wall=stamp(CLOCK_MONOTONIC)-wall;
 assert(!fflush(stdout));assert(dup2(saved,1)==1);close(saved);int status;assert(waitpid(child,&status,0)==child && status==0);free(bytes);
 printf("{\"bytes\":%d,\"direct\":%d,\"wall_ns\":%.3f,\"cpu_ns\":%.3f}\n",size,direct,(double)wall/N,(double)cpu/N);return 0;
}
