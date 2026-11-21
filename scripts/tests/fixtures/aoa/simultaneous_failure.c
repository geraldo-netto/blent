#define _POSIX_C_SOURCE 200809L
#include <assert.h>
#include <errno.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

static pthread_barrier_t failures;
static ssize_t failed_read(int fd, void *bytes, size_t size);
#define read failed_read
#define accessory_lock fixture_lock
#define main stream_main
#include "stream.c"
#undef main
#undef accessory_lock
#undef read

static void simultaneous_failure(void)
{
    int status = pthread_barrier_wait(&failures);
    assert(status == 0 || status == PTHREAD_BARRIER_SERIAL_THREAD);
}

static ssize_t failed_read(int fd, void *bytes, size_t size)
{
    (void)bytes;
    assert(fd == STDIN_FILENO && size == 65536);
    simultaneous_failure();
    errno = EIO;
    return -1;
}

int libusb_bulk_transfer(libusb_device_handle *handle, unsigned char endpoint,
                        unsigned char *bytes, int length, int *count, unsigned int timeout)
{
    (void)handle;
    (void)bytes;
    assert(endpoint == 128 && length == 65536 && timeout == 1000);
    simultaneous_failure();
    *count = 0;
    return LIBUSB_ERROR_NO_DEVICE;
}

int fixture_lock(accessory_target target)
{
    assert(target.bus == 254 && target.count == 1 && target.ports[0] == 255);
    return -1;
}

void accessory_open_selected(accessory_usb *usb, accessory_target target, accessory_selection selected)
{
    (void)target;
    assert(selected.address == 7 && selected.vendor == 6353 && selected.product == 201);
    usb->input = 128;
    usb->output_packet = 512;
}

void accessory_open(accessory_usb *usb, accessory_target target)
{
    (void)usb;
    (void)target;
    abort();
}

void accessory_transfer(accessory_usb *usb, int receive, unsigned char *bytes, int length)
{
    (void)usb;
    (void)receive;
    (void)bytes;
    (void)length;
    abort();
}

int accessory_list(const char *limit)
{
    (void)limit;
    abort();
}

static void unsafe_handler(void)
{
    static const char marker[] = "T1614_UNSAFE_EXIT_HANDLER\n";
    assert(write(STDERR_FILENO, marker, sizeof(marker) - 1) == sizeof(marker) - 1);
}

int main(void)
{
    assert(pthread_barrier_init(&failures, NULL, 2) == 0);
    assert(atexit(unsafe_handler) == 0);
    char *arguments[] = {"fixture", "--selected", "254", "255", "7", "6353", "201", NULL};
    return stream_main(7, arguments);
}
