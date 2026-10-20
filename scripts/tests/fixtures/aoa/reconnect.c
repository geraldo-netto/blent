/* T1237: USB gadget queues outlive a logical connection. Native epoch model. */
#define _POSIX_C_SOURCE 200809L
#include "accessory.h"
#include <libusb.h>
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

struct libusb_device { unsigned int epoch; };
struct libusb_context { int unused; };
struct libusb_device_handle { libusb_device *device; };
static struct libusb_device original = {0}, replacement = {1};
static struct libusb_context context;
static struct libusb_device_handle handle;
static int mode, reset_status, reset_calls, sleeps, references, opened, claimed, switched;
static int queue_is_fresh;
static struct libusb_endpoint_descriptor endpoints[] = {{2, 0x81, 512}, {2, 0x02, 512}};
static struct libusb_interface_descriptor descriptor = {255, 255, 0, 2, 0, 0, endpoints};
static struct libusb_interface interface = {&descriptor, 1};
static struct libusb_config_descriptor config = {1, &interface};

int libusb_init(libusb_context **out)
{
    *out = &context;
    return 0;
}

void libusb_exit(libusb_context *value)
{
    assert(value == &context && !opened && references == 0 && !claimed);
}

uint8_t libusb_get_bus_number(libusb_device *value)
{
    assert(value == &original || value == &replacement);
    return 3;
}

uint8_t libusb_get_device_address(libusb_device *value)
{
    return value == &original ? 7 : 8;
}

int libusb_get_port_numbers(libusb_device *value, uint8_t *ports, int count)
{
    assert((value == &original || value == &replacement) && count >= 1);
    ports[0] = 2;
    return 1;
}

ssize_t libusb_get_device_list(libusb_context *value, libusb_device ***out)
{
    static libusb_device *list[2];
    assert(value == &context);
    if (reset_calls && mode == 6 && sleeps == 1)
    {
        *out = list;
        return 0;
    }
    list[0] = &original;
    if (reset_calls && sleeps >= 2 && mode != 1)
    {
        list[0] = &replacement;
    }
    list[1] = NULL;
    *out = list;
    return 1;
}

void libusb_free_device_list(libusb_device **list, int unref)
{
    assert(list && unref == 1);
}

int libusb_open(libusb_device *value, libusb_device_handle **out)
{
    assert(!opened);
    if (value == &replacement)
    {
        assert(references == 1 || switched);
    }
    if (value == &replacement && mode == 7 && sleeps == 2)
    {
        return LIBUSB_ERROR_IO;
    }
    opened = 1;
    handle.device = value;
    *out = &handle;
    return 0;
}

void libusb_close(libusb_device_handle *value)
{
    assert(value == &handle && opened);
    if (claimed)
    {
        assert(value->device == &original && reset_calls && references == 1);
        claimed = 0;
    }
    opened = 0;
}

libusb_device *libusb_get_device(libusb_device_handle *value)
{
    assert(value == &handle && opened);
    return value->device;
}

libusb_device *libusb_ref_device(libusb_device *value)
{
    assert(value == &original && references == 0);
    references++;
    return value;
}

void libusb_unref_device(libusb_device *value)
{
    assert(value == &original && references == 1);
    references--;
}

int libusb_reset_device(libusb_device_handle *value)
{
    assert(value == &handle && opened && claimed && references == 1);
    assert(mode != 8);
    reset_calls++;
    queue_is_fresh = 1;
    if (mode == 2)
    {
        return LIBUSB_ERROR_IO;
    }
    if (mode == 3)
    {
        return LIBUSB_ERROR_NOT_FOUND;
    }
    if (mode == 4)
    {
        return LIBUSB_ERROR_NO_DEVICE;
    }
    if (mode == 9)
    {
        return reset_status;
    }
    return 0;
}

int libusb_get_device_descriptor(libusb_device *value, struct libusb_device_descriptor *out)
{
    out->idVendor = 0x18d1;
    out->idProduct = 0x2d01;
    if (value == &replacement && !switched && mode != 5)
    {
        out->idProduct = 0x4ee8;
    }
    if (value == &original && mode == 10)
    {
        out->idProduct = 0x2d00;
    }
    return 0;
}

int libusb_control_transfer(libusb_device_handle *value, uint8_t type, uint8_t request,
    uint16_t val, uint16_t index, unsigned char *data, uint16_t length, unsigned int timeout)
{
    assert(value == &handle && value->device == &replacement && val == 0 && timeout == 1000);
    (void)type;
    (void)index;
    if (request == 51)
    {
        assert(length == 2);
        data[0] = 2;
        data[1] = 0;
    }
    if (request == 53)
    {
        switched = 1;
    }
    return length;
}

int libusb_get_active_config_descriptor(libusb_device *value, struct libusb_config_descriptor **out)
{
    (void)value;
    *out = &config;
    return 0;
}

void libusb_free_config_descriptor(struct libusb_config_descriptor *value)
{
    assert(value == &config);
}

int libusb_claim_interface(libusb_device_handle *value, int number)
{
    assert(value == &handle && number == 0 && !claimed);
    if (mode == 11 && value->device == &original)
    {
        return -6;
    }
    claimed = 1;
    return 0;
}

int libusb_release_interface(libusb_device_handle *value, int number)
{
    assert(value == &handle && number == 0 && claimed);
    claimed = 0;
    return 0;
}

int libusb_bulk_transfer(libusb_device_handle *value, unsigned char endpoint,
    unsigned char *data, int length, int *transferred, unsigned int timeout)
{
    assert(value == &handle && endpoint == 0x81 && length == 8 && timeout > 0);
    const unsigned char stale[8] = {'B', 'L', 'P', 'A', 1, 1, 0, 8};
    const unsigned char fresh[8] = {'B', 'L', 'P', 'R', 1, 1, 0, 0};
    memcpy(data, queue_is_fresh ? fresh : stale, 8);
    *transferred = 8;
    return 0;
}

int nanosleep(const struct timespec *duration, struct timespec *remaining)
{
    (void)remaining;
    assert(duration->tv_sec == 0 && duration->tv_nsec == 100000000);
    assert(++sleeps <= 200);
    return 0;
}

int main(int argc, char **argv)
{
    assert(argc == 2 || argc == 3);
    mode = atoi(argv[1]);
    assert(mode >= 0 && mode <= 11);
    if (mode == 9)
    {
        assert(argc == 3);
        reset_status = atoi(argv[2]);
    }
    accessory_usb usb = {0};
    accessory_target target = {.bus = 3, .ports = {2}, .count = 1};
    accessory_selection selected = {.address = 7, .vendor = 0x18d1, .product = 0x2d01};
    if (mode == 8)
    {
        selected.address = 6;
    }
    if (mode == 10)
    {
        selected.product = 0x2d00;
    }
    accessory_open_selected(&usb, target, selected);
    unsigned char bytes[8];
    accessory_transfer(&usb, 1, bytes, sizeof(bytes));
    const unsigned char expected[8] = {'B', 'L', 'P', 'R', 1, 1, 0, 0};
    assert(!memcmp(bytes, expected, sizeof(expected)) && "T1237 old connection bytes survived reopening");
    accessory_close(&usb);
    return 0;
}
