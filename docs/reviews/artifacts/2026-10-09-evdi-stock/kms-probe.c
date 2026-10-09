/* Guest-only stock API probe. No production mitigation or compositor claim. */
#include <xf86drm.h>
#include <xf86drmMode.h>
#include <drm_fourcc.h>
#include "evdi_lib.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <poll.h>
#include <sys/mman.h>
static int ready, modes;
static void mode_cb(struct evdi_mode m,void *p){(void)p;modes++;printf("mode %dx%d %.4s\n",m.width,m.height,(char*)&m.pixel_format);}
static void ready_cb(int b,void*p){(void)p;(void)b;ready++;}
static struct evdi_event_context events={.mode_changed_handler=mode_cb,.update_ready_handler=ready_cb};
static void drain(evdi_handle h,int ms){struct pollfd p={evdi_get_event_ready(h),POLLIN,0};while(poll(&p,1,ms)>0){evdi_handle_events(h,&events);ms=0;}}
static void grab(evdi_handle h,const char *label,uint32_t *pixels){struct evdi_rect rects[64];int n=64;evdi_grab_pixels(h,rects,&n);printf("%s rects=%d modes=%d ready=%d pixel=%08x\n",label,n,modes,ready,pixels[10*640+10]);}
static struct drm_mode_create_dumb buffer(int fd,int w,int h,uint32_t color){struct drm_mode_create_dumb b={.width=w,.height=h,.bpp=32};if(drmIoctl(fd,DRM_IOCTL_MODE_CREATE_DUMB,&b))exit(70);struct drm_mode_map_dumb map={.handle=b.handle};if(drmIoctl(fd,DRM_IOCTL_MODE_MAP_DUMB,&map))exit(71);uint32_t *p=mmap(NULL,b.size,PROT_READ|PROT_WRITE,MAP_SHARED,fd,map.offset);if(p==MAP_FAILED)exit(72);for(unsigned i=0;i<b.size/4;i++)p[i]=color;munmap(p,b.size);return b;}
int main(void){
 setbuf(stdout,NULL);char cmd[4096]={0};FILE*f=fopen("/proc/cmdline","r");if(!f||!fgets(cmd,sizeof(cmd),f)||!strstr(cmd,"blent-evdi-review=1"))return 64;fclose(f);
 int fd=-1,card=0;drmVersionPtr v=NULL;for(;card<8;card++){char path[64];snprintf(path,sizeof(path),"/dev/dri/card%d",card);fd=open(path,O_RDWR);v=drmGetVersion(fd);if(v&&!strcmp(v->name,"evdi"))break;if(v)drmFreeVersion(v);v=NULL;if(fd>=0)close(fd);}if(!v||strcmp(v->name,"evdi"))return 65;printf("KMS stock %d.%d.%d\n",v->version_major,v->version_minor,v->version_patchlevel);drmFreeVersion(v);
 unsigned char edid[128];f=fopen("/edid.bin","rb");if(!f||fread(edid,1,128,f)!=128)return 66;fclose(f);
 evdi_handle h=evdi_open(card);if(!h)return 67;evdi_connect(h,edid,128,0);
 drmModeResPtr res=drmModeGetResources(fd);if(!res||!res->count_connectors||!res->count_crtcs)return 68;
 drmModeConnectorPtr c=drmModeGetConnector(fd,res->connectors[0]);if(!c||!c->count_modes)return 69;
 printf("connector modes=%d first=%dx%d\n",c->count_modes,c->modes[0].hdisplay,c->modes[0].vdisplay);
 struct drm_mode_create_dumb b=buffer(fd,640,480,0xff000080);uint32_t handles[4]={b.handle},pitches[4]={b.pitch},offsets[4]={0},fb=0;
 if(drmModeAddFB2(fd,640,480,DRM_FORMAT_XRGB8888,handles,pitches,offsets,&fb,0))return 73;
 if(drmModeSetCrtc(fd,res->crtcs[0],fb,0,0,&c->connector_id,1,&c->modes[0])){perror("setcrtc");return 74;}
 drain(h,300);uint32_t *pixels=calloc(640*480,4);struct evdi_buffer out={.id=0,.buffer=pixels,.width=640,.height=480,.stride=640*4};evdi_register_buffer(h,out);
 printf("initial immediate=%d\n",evdi_request_update(h,0));drain(h,100);grab(h,"initial",pixels);
 printf("T748 idle request=%d\n",evdi_request_update(h,0));drain(h,300);grab(h,"T748 watchdog",pixels);printf("T748 retry=%d\n",evdi_request_update(h,0));
 drmSetClientCap(fd,DRM_CLIENT_CAP_UNIVERSAL_PLANES,1);drmModePlaneResPtr ps=drmModeGetPlaneResources(fd);uint32_t cp=ps->planes[1];
 struct drm_mode_create_dumb cursor=buffer(fd,64,64,0xffffffff);uint32_t ch[4]={cursor.handle},cs[4]={cursor.pitch},cf=0;
 printf("cursor fb=%d\n",drmModeAddFB2(fd,64,64,DRM_FORMAT_ARGB8888,ch,cs,offsets,&cf,0));
 printf("cursor set=%d\n",drmModeSetPlane(fd,cp,res->crtcs[0],cf,0,0,0,64,64,0,0,64<<16,64<<16));drain(h,300);grab(h,"T753 cursor shown",pixels);
 printf("cursor hide=%d\n",drmModeSetPlane(fd,cp,res->crtcs[0],0,0,0,0,0,0,0,0,0,0));printf("T753 hide request=%d\n",evdi_request_update(h,0));drain(h,300);grab(h,"T753 cursor hidden",pixels);
 evdi_disconnect(h);evdi_connect(h,edid,128,0);drain(h,300);printf("T749 reconnect request=%d\n",evdi_request_update(h,0));drain(h,300);grab(h,"T749 reconnect no modeset",pixels);
 evdi_unregister_buffer(h,0);evdi_disconnect(h);evdi_close(h);free(pixels);close(fd);return 0;
}
