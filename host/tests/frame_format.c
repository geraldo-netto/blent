/* T752: bounded format/damage inputs, lifecycle and allocation failure. */
static int t752_contains(int a, int b, int point) {
    return (point >= a && point < b) || (point >= b && point < a);
}

static void t752_check_region(struct evdi_rect rect) {
    for (int y=0; y<6; y++) for (int x=0; x<6; x++) {
        int inside=t752_contains(rect.x1,rect.x2,x) && t752_contains(rect.y1,rect.y2,y);
        uint32_t pixel=*(uint32_t *)(g_capture.framebuffer+y*g_capture.mode_stride+4*x);
        assert(pixel==(inside ? 0x7fc85a14u : 0x18181818u));
    }
    for (int y=0; y<6; y++) for (int x=24; x<g_capture.mode_stride; x++)
        assert(g_capture.framebuffer[y*g_capture.mode_stride+x]==0x18 && "T752: preserve stride padding");
}

static void t752_format_inputs(void) {
    const int depths[]={INT_MIN,0,8,16,24,31,33,64,INT_MAX};
    atomic_int running=1;
    capture_context_t capture=CAPTURE_INITIALIZER(&g_frames,&g_conversion,&running);
    for (unsigned i=0;i<sizeof(depths)/sizeof(depths[0]);i++)
        assert(!validate_frame_format(&capture,(struct evdi_mode){6,6,60,depths[i],0x34324258}));
    const unsigned formats[]={0,UINT_MAX,0x30335258,0x30335241,0x36314752};
    for (unsigned i=0;i<sizeof(formats)/sizeof(formats[0]);i++)
        assert(!validate_frame_format(&capture,(struct evdi_mode){6,6,60,32,formats[i]}));
}

static void test_t752_bounds(void) {
    frame_exchange_init(&g_frames); g_running=1;
    on_mode_changed((struct evdi_mode){6,6,60,32,0x34324241},&g_capture);
    assert(g_capture.have_mode);
    for (int y=0;y<6;y++) for (int x=0;x<6;x++)
        *(uint32_t *)(g_capture.native_framebuffer+y*g_capture.mode_stride+4*x)=0x7f145ac8;
    const int endpoints[]={INT_MIN,-7,-1,0,3,6,7,INT_MAX};
    for (unsigned i=0;i<8;i++) for (unsigned j=0;j<8;j++) {
        struct evdi_rect rect={endpoints[i],endpoints[j],3,3};
        memset(g_capture.framebuffer,0x18,g_capture.fb_size);
        normalize_framebuffer(&g_capture,&rect,1);
        t752_check_region(rect);
    }
    memset(g_capture.framebuffer,0x18,g_capture.fb_size);
    normalize_framebuffer(&g_capture,NULL,INT_MIN);
    normalize_framebuffer(&g_capture,NULL,0);
    t752_check_region((struct evdi_rect){0});
    normalize_framebuffer(&g_capture,NULL,INT_MAX);
    t752_check_region((struct evdi_rect){0,0,6,6});
    t752_format_inputs();
    free(g_capture.framebuffer);free(g_capture.native_framebuffer);
    frame_exchange_free(&g_frames);pthread_cond_destroy(&g_frames.ready);
}

static void test_t752_allocation(void) {
    frame_exchange_init(&g_frames);g_running=1;
    allocation_countdown=1;
    on_mode_changed((struct evdi_mode){6,6,60,32,0x34324258},&g_capture);
    assert(!g_capture.have_mode && !g_capture.buffer_registered && g_capture.capture_failed);
    assert(!g_running && !g_capture.native_framebuffer);
    free(g_capture.framebuffer);
    frame_exchange_free(&g_frames);pthread_cond_destroy(&g_frames.ready);
}

static void test_t780_full_rect_capacity(void) {
    frame_exchange_init(&g_frames); g_running=1;
    on_mode_changed((struct evdi_mode){6,6,60,32,0x34324258},&g_capture);
    assert(g_capture.have_mode);
    for (int y=0;y<6;y++) for (int x=0;x<6;x++)
        *(uint32_t *)(g_capture.native_framebuffer+y*g_capture.mode_stride+4*x)=0x7f145ac8;
    memset(g_capture.framebuffer,0x18,g_capture.fb_size);
    struct evdi_rect rectangles[64];
    for (unsigned i=0;i<64;i++) rectangles[i]=(struct evdi_rect){0,0,1,1};
    normalize_framebuffer(&g_capture,rectangles,64);
    assert(*(uint32_t *)(g_capture.framebuffer+5*g_capture.mode_stride+20)==0x7fc85a14u
           && "T780: exact rectangle capacity requires full-frame normalization");
    t752_check_region((struct evdi_rect){0,0,6,6});
    free(g_capture.framebuffer); free(g_capture.native_framebuffer);
    frame_exchange_free(&g_frames); pthread_cond_destroy(&g_frames.ready);
}

static void test_t752_cursor(void) {
    frame_exchange_init(&g_frames);g_running=1;
    on_mode_changed((struct evdi_mode){8,8,60,32,0x34324258},&g_capture);
    memset(g_capture.native_framebuffer,0x18,g_capture.fb_size);
    *(uint32_t*)g_capture.native_framebuffer=0xff0000ff; /* native RGBA red */
    struct evdi_cursor_set event={.enabled=1,.width=1,.height=1,.stride=4,.buffer_length=4,.pixel_format=0x34325241};
    event.buffer=malloc(4);assert(event.buffer);*event.buffer=0xffffffff;
    on_cursor_set(event,&g_capture);
    t752_grabs=1;t752_count=1;t752_rects[0]=(struct evdi_rect){0,0,8,8};
    grab_now(&g_capture);
    assert(*(uint32_t*)g_capture.framebuffer==0xffffffff);
    on_cursor_set((struct evdi_cursor_set){0},&g_capture);
    assert(*(uint32_t*)g_capture.framebuffer==0xffff0000);
    unsigned char *registered=t752_registered.buffer;
    on_mode_changed((struct evdi_mode){8,8,60,32,0x34324241},&g_capture);
    assert(t752_registered.buffer==registered && "T752: alpha/X change keeps equivalent channel order");
    on_mode_changed((struct evdi_mode){10,10,60,32,0x34324258},&g_capture);
    assert(g_capture.have_mode && g_capture.mode_w==10);
    on_mode_changed((struct evdi_mode){8,8,60,32,0x34325241},&g_capture);
    assert(g_capture.have_mode && !g_capture.native_framebuffer);
    free(g_capture.framebuffer);
    frame_exchange_free(&g_frames);pthread_cond_destroy(&g_frames.ready);
}

static void test_t752_shared(void) {
    raw_ring_t ring=RAW_RING_INITIALIZER;
    int pair[2];assert(socketpair(AF_UNIX,SOCK_SEQPACKET|SOCK_NONBLOCK,0,pair)==0);
    assert(raw_ring_init(&ring,pair[0]));
    frame_exchange_init(&g_frames);g_capture.raw_ring=&ring;g_running=1;
    on_mode_changed((struct evdi_mode){8,8,60,32,0x34324258},&g_capture);
    raw_ring_t hello=ring;hello.nonce=8;t418_control(&hello,pair[1],4);
    mock_monotonic_ms=1000;assert(service_shared_capture(&g_capture));
    unsigned char leased[96];memcpy(leased,ring.memory+RAW_CONTROL_BYTES,sizeof(leased));
    for (int y=0;y<8;y++) for (int x=0;x<8;x++)
        *(uint32_t *)(g_capture.native_framebuffer+y*g_capture.mode_stride+4*x)=0xff0000ff;
    t752_grabs=1;t752_count=1;t752_rects[0]=(struct evdi_rect){0,0,8,8};
    grab_now(&g_capture);mock_monotonic_ms=1020;
    assert(service_shared_capture(&g_capture));
    assert(!memcmp(leased,ring.memory+RAW_CONTROL_BYTES,sizeof(leased)) && "T752: preserve leased pixels");
    unsigned char *fresh=ring.memory+RAW_CONTROL_BYTES+ring.slot_bytes;
    assert(fresh[0]==63 && fresh[64]==102 && fresh[65]==240);
    raw_ring_close(&ring);close(pair[1]);g_capture.raw_ring=NULL;
    free(g_capture.framebuffer);free(g_capture.native_framebuffer);
    frame_exchange_free(&g_frames);pthread_cond_destroy(&g_frames.ready);
}
