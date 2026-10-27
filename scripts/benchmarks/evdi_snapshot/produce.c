#define _GNU_SOURCE
#include "pattern.h"
#include <xf86drm.h>
#include <xf86drmMode.h>
#include <drm_fourcc.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <poll.h>
#include <sys/mman.h>
static unsigned flips;
static void flip(int fd,unsigned sequence,unsigned sec,unsigned usec,void *data) {
    (void)fd;flips++;printf("SNAP_FLIP %lld frame=%lu sequence=%u drm_us=%llu\n",stamp_us(),(unsigned long)data,sequence,(unsigned long long)sec*1000000+usec);
}
static int property(int fd,uint32_t object,uint32_t type,const char *name,uint64_t *value) {
    drmModeObjectPropertiesPtr props=drmModeObjectGetProperties(fd,object,type);assert(props);int id=0;
    for (uint32_t i=0;i<props->count_props;i++) {
        drmModePropertyPtr p=drmModeGetProperty(fd,props->props[i]);assert(p);
        if (!strcmp(p->name,name)) { id=(int)p->prop_id;if(value)*value=props->prop_values[i]; }
        drmModeFreeProperty(p);
    }
    drmModeFreeObjectProperties(props);return id;
}
typedef struct
{
    uint32_t connector, crtc;
    drmModeModeInfo mode;
} snapshot_output;

static int resource_extent(const drmModeRes *resources)
{
    return resources && resources->count_connectors > 0 && resources->connectors &&
        resources->count_crtcs > 0 && resources->crtcs;
}

static int connector_extent(const drmModeConnector *connector)
{
    return connector && connector->connection == DRM_MODE_CONNECTED &&
        connector->count_modes > 0 && connector->modes;
}

/* Borrow each inventory only for this observation; copy the admitted values.
 * Discovery cannot authorize indexing a later topology snapshot. */
static int read_output(int fd, snapshot_output *output)
{
    drmModeResPtr resources = drmModeGetResources(fd);
    drmModeConnectorPtr connector = NULL;
    if (resource_extent(resources))
        connector = drmModeGetConnector(fd, resources->connectors[0]);
    int valid = connector_extent(connector);
    if (valid)
        *output = (snapshot_output){connector->connector_id, resources->crtcs[0], connector->modes[0]};
    drmModeFreeConnector(connector);
    drmModeFreeResources(resources);
    return valid;
}

static int evdi_card(int fd)
{
    drmVersionPtr version = drmGetVersion(fd);
    int evdi = version && !strcmp(version->name, "evdi");
    drmFreeVersion(version);
    return evdi;
}

static int connected_card(void)
{
    for (int card = 0; card < 8; card++)
    {
        char path[64];
        snprintf(path, sizeof(path), "/dev/dri/card%d", card);
        int fd = open(path, O_RDWR);
        if (fd < 0)
            continue;
        snapshot_output output;
        if (evdi_card(fd) && read_output(fd, &output))
            return fd;
        close(fd);
    }
    return -1;
}
static uint32_t primary(int fd) {
    drmModePlaneResPtr planes=drmModeGetPlaneResources(fd);assert(planes);uint32_t found=0;
    for (uint32_t i=0;i<planes->count_planes;i++) {
        uint64_t type=0;property(fd,planes->planes[i],DRM_MODE_OBJECT_PLANE,"type",&type);
        if(type==DRM_PLANE_TYPE_PRIMARY)found=planes->planes[i];
    }
    drmModeFreePlaneResources(planes);assert(found);return found;
}
/* Each FB is immutable until process exit. No producer buffer-reuse tearing. */
static uint32_t framebuffer(int fd,unsigned frame) {
    struct drm_mode_create_dumb b={.width=SW,.height=SH,.bpp=32};assert(!drmIoctl(fd,DRM_IOCTL_MODE_CREATE_DUMB,&b));
    struct drm_mode_map_dumb map={.handle=b.handle};assert(!drmIoctl(fd,DRM_IOCTL_MODE_MAP_DUMB,&map));
    unsigned char *p=mmap(NULL,b.size,PROT_READ|PROT_WRITE,MAP_SHARED,fd,map.offset);assert(p!=MAP_FAILED);
    for (int y=0;y<SH;y++) for(int x=0;x<SW;x++) {
        unsigned g=expected(frame,x,y);uint32_t pixel=0xff000000u|g*0x010101u;memcpy(p+(size_t)y*b.pitch+x*4,&pixel,4);
    }
    munmap(p,b.size);uint32_t handles[4]={b.handle},pitches[4]={b.pitch},offsets[4]={0},fb=0;
    assert(!drmModeAddFB2(fd,SW,SH,DRM_FORMAT_XRGB8888,handles,pitches,offsets,&fb,0));return fb;
}
static void drain(int fd,int timeout) {
    struct pollfd p={fd,POLLIN,0};drmEventContext events={.version=2,.page_flip_handler=flip};
    if(poll(&p,1,timeout)>0)assert(!drmHandleEvent(fd,&events));
}
static void submit(int fd,uint32_t plane,uint32_t fb,unsigned frame) {
    unsigned tile=(frame-1)%STILES;int x=(int)(tile%10)*64,y=32+(int)(tile/10)*56;
    struct drm_mode_rect rects[63]={{0,0,SW,16},{x,y,x+64,y+56}};uint32_t blob;
    int count=getenv("SNAP_FRAGMENTED") ? 63 : 2;
    for (int i=2;i<count;i++) rects[i]=rects[i%2];
    assert(!drmModeCreatePropertyBlob(fd,rects,(size_t)count*sizeof(*rects),&blob));
    drmModeAtomicReqPtr request=drmModeAtomicAlloc();assert(request);
    assert(drmModeAtomicAddProperty(request,plane,property(fd,plane,DRM_MODE_OBJECT_PLANE,"FB_ID",NULL),fb)>=0);
    assert(drmModeAtomicAddProperty(request,plane,property(fd,plane,DRM_MODE_OBJECT_PLANE,"FB_DAMAGE_CLIPS",NULL),blob)>=0);
    int ret=-1;
    uint32_t flags=DRM_MODE_ATOMIC_NONBLOCK;
    if(frame<=SN/2)flags|=DRM_MODE_PAGE_FLIP_EVENT;
    for(int tries=0;tries<100;tries++) {
        ret=drmModeAtomicCommit(fd,request,flags,(void *)(uintptr_t)frame);
        if(!ret || errno!=EBUSY)break;
        drain(fd,20);
    }
    if(ret)perror("atomic commit");assert(!ret);
    printf("SNAP_COMMIT %lld frame=%u\n",stamp_us(),frame);
    drmModeAtomicFree(request);assert(!drmModeDestroyPropertyBlob(fd,blob));
}
int main(void)
{
    guest_only();
    setbuf(stdout, NULL);
    int fd = connected_card();
    if (fd < 0)
        return 2;
    snapshot_output output;
    if (!read_output(fd, &output))
    {
        close(fd);
        return 2;
    }
    assert(!drmSetClientCap(fd,DRM_CLIENT_CAP_ATOMIC,1));uint32_t plane=primary(fd);
    uint32_t initial=framebuffer(fd,0);assert(!drmModeSetCrtc(fd,output.crtc,initial,0,0,&output.connector,1,&output.mode));
    usleep(100000);
    for(unsigned frame=1;frame<=SN;frame++)
    {
        uint32_t fb=framebuffer(fd,frame);submit(fd,plane,fb,frame);
        if(frame<=SN/2)drain(fd,100);else drain(fd,0);
    }
    long long end=stamp_us()+3000000;while(stamp_us()<end)drain(fd,50);
    printf("SNAP_PRODUCER frames=%d flips=%u\n",SN,flips);
    /* Keep immutable scanout alive through final consumer observation. */
    sleep(10);close(fd);return flips!=SN/2;
}
