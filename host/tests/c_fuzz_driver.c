/* T767: optional clang libFuzzer entry; same boundaries run in cargo test. */
#define BLENT_C_FUZZ_DRIVER
#include "evdi_helper_test.c"
int LLVMFuzzerTestOneInput(const unsigned char *bytes, size_t size) {
    unsigned char data[128] = {0};
    if (size > sizeof(data)) size = sizeof(data);
    memcpy(data, bytes, size);
    fuzz_boundary_input(data);
    return 0;
}
