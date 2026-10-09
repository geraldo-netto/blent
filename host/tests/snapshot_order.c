/* T750: coherent driver snapshots stay coherent through skipped publications,
 * coalesced/reordered/duplicate damage and retained writer leases. */
#include "../../scripts/benchmarks/evdi_snapshot/pattern.h"

static void t750_paint(unsigned frame,struct evdi_rect rect) {
    for (int y=rect.y1;y<rect.y2;y++) for(int x=rect.x1;x<rect.x2;x++) {
        unsigned g=expected(frame,x,y);
        uint32_t pixel=0xff000000u|g*0x010101u;
        memcpy(g_capture.framebuffer+(size_t)y*g_capture.mode_stride+x*4,&pixel,4);
    }
}

static void t750_coalesce(unsigned frame) {
    unsigned tile=(frame-1)%STILES;
    int x=(int)(tile%10)*64,y=32+(int)(tile/10)*56;
    struct evdi_rect rects[2]={{0,0,SW,16},{x,y,x+64,y+56}};
    /* Reverse metadata order on alternating frames. Pixel update order unchanged. */
    for(int i=0;i<2;i++) {
        t750_paint(frame,rects[i]);
        t752_rects[t752_count++]=rects[i^(frame&1)];
    }
}

static void t750_check_latest(unsigned wanted) {
    int frame;
    assert(!check_image(g_frames.latest,SW,1,&frame));
    assert(frame==(int)wanted && "T750: coalesced damage lost the latest complete snapshot");
}

static void t750_publish_batch(unsigned frame) {
    t752_grabs=1;
    on_update_ready(0,&g_capture);
    t750_check_latest(frame);
    /* Duplicate ready callback with empty damage must preserve the snapshot. */
    t752_count=0;on_update_ready(0,&g_capture);t750_check_latest(frame);
}

static void t750_oracle_mutations(unsigned char *pixels) {
    int frame;
    assert(!check_image(pixels,SW,1,&frame) && frame==SN);
    for(unsigned i=0;i<32;i++) {
        size_t offset=(size_t)i*15401%SSIZE;
        pixels[offset]^=1;
        assert(check_image(pixels,SW,1,&frame) && "T750: snapshot oracle failed to detect corrupted pixels");
        pixels[offset]^=1;
    }
}

static void test_t750_snapshot_order(void) {
    frame_exchange_init(&g_frames);g_running=1;
    on_mode_changed((struct evdi_mode){SW,SH,60,32,0x34325258},&g_capture);
    t750_paint(0,(struct evdi_rect){0,0,SW,SH});
    t752_rects[0]=(struct evdi_rect){0,0,SW,SH};t752_count=1;t750_publish_batch(0);
    frame_cursor_t cursor={0};frame_lease_t lease={0};
    unsigned char *held=malloc(SSIZE);assert(held);
    for(unsigned frame=1;frame<=SN;frame++) {
        t750_coalesce(frame);
        if(frame%8)continue;
        t750_publish_batch(frame);
        if(lease.data)assert(!memcmp(lease.data,held,SSIZE) && "T750: publication mutated retained snapshot");
        if(frame%24)continue;
        frame_exchange_release(&g_frames);
        assert(frame_exchange_claim(&g_frames,&cursor,&g_running,&(struct timespec){0},&lease)==1);
        memcpy(held,lease.data,SSIZE);
    }
    t750_oracle_mutations(held);
    frame_exchange_release(&g_frames);free(held);t497_free_capture();
}
