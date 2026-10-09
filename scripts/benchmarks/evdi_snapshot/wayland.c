#define _GNU_SOURCE
#include "pattern.h"
#include "xdg-shell.h"
#include "presentation-time.h"
#include <wayland-client.h>
#include <sys/mman.h>
#include <unistd.h>
#include <poll.h>
struct client {
    struct wl_display *display;
    struct wl_compositor *compositor;
    struct wl_shm *shm;
    struct xdg_wm_base *shell;
    struct wp_presentation *presentation;
    struct wl_surface *surface;
    int configured,frame_ready;
    unsigned presented,discarded;
};
static struct client client;
static void ping(void *data,struct xdg_wm_base *shell,uint32_t serial) { (void)data;xdg_wm_base_pong(shell,serial); }
static const struct xdg_wm_base_listener shell_listener={ping};
static void clock_id(void *data,struct wp_presentation *presentation,uint32_t id) {
    (void)data;(void)presentation;printf("SNAP_WL_CLOCK %u\n",id);assert(id==CLOCK_MONOTONIC);
}
static const struct wp_presentation_listener presentation_listener={clock_id};
static void global(void *data,struct wl_registry *registry,uint32_t name,const char *interface,uint32_t version) {
    (void)data;(void)version;
    if(!strcmp(interface,"wl_compositor"))client.compositor=wl_registry_bind(registry,name,&wl_compositor_interface,4);
    if(!strcmp(interface,"wl_shm"))client.shm=wl_registry_bind(registry,name,&wl_shm_interface,1);
    if(!strcmp(interface,"xdg_wm_base"))client.shell=wl_registry_bind(registry,name,&xdg_wm_base_interface,1);
    if(!strcmp(interface,"wp_presentation"))client.presentation=wl_registry_bind(registry,name,&wp_presentation_interface,1);
}
static void global_remove(void *data,struct wl_registry *registry,uint32_t name) { (void)data;(void)registry;(void)name; }
static const struct wl_registry_listener registry_listener={global,global_remove};
static void configure(void *data,struct xdg_surface *surface,uint32_t serial) {
    (void)data;xdg_surface_ack_configure(surface,serial);client.configured=1;
}
static const struct xdg_surface_listener surface_listener={configure};
static void top_configure(void *data,struct xdg_toplevel *top,int32_t width,int32_t height,struct wl_array *states) {
    (void)data;(void)top;(void)states;assert(!width || width==SW);assert(!height || height==SH);
}
static void top_close(void *data,struct xdg_toplevel *top) { (void)data;(void)top;exit(75); }
static const struct xdg_toplevel_listener top_listener={.configure=top_configure,.close=top_close};
static void frame_done(void *data,struct wl_callback *callback,uint32_t time) {
    (void)data;(void)time;wl_callback_destroy(callback);client.frame_ready=1;
}
static const struct wl_callback_listener frame_listener={frame_done};
static void sync_output(void *data,struct wp_presentation_feedback *feedback,struct wl_output *output) { (void)data;(void)feedback;(void)output; }
static void presented(void *data,struct wp_presentation_feedback *feedback,uint32_t hi,uint32_t lo,uint32_t ns,uint32_t refresh,uint32_t seq_hi,uint32_t seq_lo,uint32_t flags) {
    unsigned long long timestamp=(((unsigned long long)hi<<32)|lo)*1000000+ns/1000;
    printf("SNAP_PRESENT %lld frame=%lu presented_us=%llu refresh_ns=%u sequence=%llu flags=%u\n",stamp_us(),(unsigned long)data,timestamp,refresh,((unsigned long long)seq_hi<<32)|seq_lo,flags);
    client.presented++;wp_presentation_feedback_destroy(feedback);
}
static void discarded(void *data,struct wp_presentation_feedback *feedback) {
    printf("SNAP_DISCARD %lld frame=%lu\n",stamp_us(),(unsigned long)data);client.discarded++;wp_presentation_feedback_destroy(feedback);
}
static const struct wp_presentation_feedback_listener feedback_listener={sync_output,presented,discarded};
static void setup(void) {
    client.display=wl_display_connect(NULL);assert(client.display);
    struct wl_registry *registry=wl_display_get_registry(client.display);wl_registry_add_listener(registry,&registry_listener,NULL);
    assert(wl_display_roundtrip(client.display)>=0);assert(client.compositor && client.shm && client.shell && client.presentation);
    xdg_wm_base_add_listener(client.shell,&shell_listener,NULL);wp_presentation_add_listener(client.presentation,&presentation_listener,NULL);
    client.surface=wl_compositor_create_surface(client.compositor);
    struct xdg_surface *surface=xdg_wm_base_get_xdg_surface(client.shell,client.surface);xdg_surface_add_listener(surface,&surface_listener,NULL);
    struct xdg_toplevel *top=xdg_surface_get_toplevel(surface);xdg_toplevel_add_listener(top,&top_listener,NULL);
    xdg_toplevel_set_app_id(top,"blent.snapshot.fixture");xdg_toplevel_set_fullscreen(top,NULL);wl_surface_commit(client.surface);
    while(!client.configured)assert(wl_display_dispatch(client.display)>=0);
}
static void paint(unsigned char *pixels,unsigned frame) {
    for(int y=0;y<SH;y++) for(int x=0;x<SW;x++) {
        unsigned g=expected(frame,x,y);uint32_t pixel=0xff000000u|g*0x010101u;
        memcpy(pixels+(size_t)y*SW*4+x*4,&pixel,4);
    }
}
static void submit(struct wl_buffer *buffer,unsigned frame) {
    struct wp_presentation_feedback *feedback=wp_presentation_feedback(client.presentation,client.surface);
    wp_presentation_feedback_add_listener(feedback,&feedback_listener,(void *)(uintptr_t)frame);
    client.frame_ready=0;wl_callback_add_listener(wl_surface_frame(client.surface),&frame_listener,NULL);
    wl_surface_attach(client.surface,buffer,0,0);
    if(!frame)wl_surface_damage_buffer(client.surface,0,0,SW,SH);
    else {
        unsigned tile=(frame-1)%STILES;
        wl_surface_damage_buffer(client.surface,0,0,SW,16);
        wl_surface_damage_buffer(client.surface,(int)(tile%10)*64,32+(int)(tile/10)*56,64,56);
    }
    wl_surface_commit(client.surface);printf("SNAP_WL_COMMIT %lld frame=%u\n",stamp_us(),frame);
    while(!client.frame_ready)assert(wl_display_dispatch(client.display)>=0);
}
static void hold(void) {
    long long end=stamp_us()+10000000;
    while(stamp_us()<end) {
        assert(wl_display_dispatch_pending(client.display)>=0);assert(wl_display_flush(client.display)>=0);
        struct pollfd p={wl_display_get_fd(client.display),POLLIN,0};
        if(poll(&p,1,100)>0)assert(wl_display_dispatch(client.display)>=0);
    }
}
int main(void) {
    guest_only();setbuf(stdout,NULL);setup();
    size_t size=(size_t)(SN+1)*SW*SH*4;int fd=memfd_create("blent-snapshots",MFD_CLOEXEC);assert(fd>=0 && !ftruncate(fd,(off_t)size));
    unsigned char *pixels=mmap(NULL,size,PROT_READ|PROT_WRITE,MAP_SHARED,fd,0);assert(pixels!=MAP_FAILED);
    struct wl_shm_pool *pool=wl_shm_create_pool(client.shm,fd,(int)size);
    for(unsigned frame=0;frame<=SN;frame++) {
        size_t offset=(size_t)frame*SW*SH*4;paint(pixels+offset,frame);
        struct wl_buffer *buffer=wl_shm_pool_create_buffer(pool,(int)offset,SW,SH,SW*4,WL_SHM_FORMAT_XRGB8888);
        submit(buffer,frame);
    }
    hold();printf("SNAP_WL_END %lld presented=%u discarded=%u\n",stamp_us(),client.presented,client.discarded);
    wl_display_disconnect(client.display);munmap(pixels,size);close(fd);return client.presented+client.discarded!=SN+1;
}
