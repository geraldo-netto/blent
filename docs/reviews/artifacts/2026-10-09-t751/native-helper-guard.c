#define main blent_helper_main
#include "/backups/disk2/projects/blent/host/evdi/evdi_helper.c"
#undef main
int main(void) {
 if (getpid() == 1 || !Xorg_running()) return 64;
 return request_evdi_device() ? 1 : 0;
}
