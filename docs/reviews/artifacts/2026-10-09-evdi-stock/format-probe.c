/* Research probe, guest only. Never execute against the host display. */
#include <xf86drm.h>
#include <xf86drmMode.h>
#include <drm_fourcc.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include "probe_device.h"
static void probe_planes(int fd) {
 drmSetClientCap(fd,DRM_CLIENT_CAP_UNIVERSAL_PLANES,1);
 drmModePlaneResPtr planes=drmModeGetPlaneResources(fd);
 if (!planes) exit(66);
 for (unsigned i=0;i<planes->count_planes;i++) {
  drmModePlanePtr p=drmModeGetPlane(fd,planes->planes[i]);
  if (!p) exit(67);
  printf("T752 plane=%u formats=",p->plane_id);
  for(unsigned j=0;j<p->count_formats;j++) printf("%.4s ",(char*)&p->formats[j]);
  puts("");drmModeFreePlane(p);
 }
 drmModeFreePlaneResources(planes);
}
int main(void) {
 probe_require_guest();
 int card; drmVersionPtr v=NULL;int fd=probe_open_evdi(&card,&v);
 printf("T752 driver=%s %d.%d.%d\n",v->name,v->version_major,v->version_minor,v->version_patchlevel);
 drmFreeVersion(v);probe_planes(fd);
 struct drm_mode_create_dumb b={.width=64,.height=64,.bpp=32};
 if (drmIoctl(fd,DRM_IOCTL_MODE_CREATE_DUMB,&b)) return 68;
 uint32_t handles[4]={b.handle},pitches[4]={b.pitch},offsets[4]={0};
 uint32_t formats[]={DRM_FORMAT_XRGB8888,DRM_FORMAT_ARGB8888,DRM_FORMAT_XRGB2101010,DRM_FORMAT_ARGB2101010};
 for(unsigned i=0;i<4;i++) {
  uint32_t fb=0;errno=0;int rc=drmModeAddFB2(fd,64,64,formats[i],handles,pitches,offsets,&fb,0);
  printf("T752 ADDFB2 %.4s rc=%d errno=%d fb=%u\n",(char*)&formats[i],rc,errno,fb);
  if(!rc)drmModeRmFB(fd,fb);
 }
 struct drm_mode_destroy_dumb d={.handle=b.handle};drmIoctl(fd,DRM_IOCTL_MODE_DESTROY_DUMB,&d);
 close(fd);return 0;
}
