/* T1338 descriptor boundary model; no USB effects or native ownership. */
#include "accessory.h"
#include <libusb.h>
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

static struct libusb_interface_descriptor alternatives[2];
static struct libusb_interface interfaces[2];
static struct libusb_config_descriptor config;
static int status, freed;

int libusb_get_active_config_descriptor(libusb_device *device, struct libusb_config_descriptor **out)
{
    assert(device == NULL);
    if (status)
    {
        return status;
    }
    *out = &config;
    return 0;
}

void libusb_free_config_descriptor(struct libusb_config_descriptor *value)
{
    assert(value == &config && freed == 0);
    freed++;
}

libusb_device *libusb_get_device(libusb_device_handle *handle)
{
    (void)handle;
    abort();
}

int main(int argc, char **argv)
{
    assert(argc == 5);
    alternatives[1].bInterfaceClass = (uint8_t)atoi(argv[1]);
    alternatives[1].bInterfaceSubClass = (uint8_t)atoi(argv[2]);
    alternatives[1].bInterfaceProtocol = (uint8_t)atoi(argv[3]);
    status = atoi(argv[4]);
    interfaces[1] = (struct libusb_interface){alternatives, 2};
    config = (struct libusb_config_descriptor){2, interfaces};
    int result = accessory_availability(NULL);
    assert(freed == (status == 0));
    printf("%d\n", result);
    return 0;
}
