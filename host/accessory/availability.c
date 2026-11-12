/* T1338: device-mode reservation; descriptor reads never claim USB interfaces. */
#include "accessory.h"
#include <libusb.h>
#include <stddef.h>

static int debugging_interface(const struct libusb_interface_descriptor *descriptor)
{
    return descriptor->bInterfaceClass == 0xff &&
           descriptor->bInterfaceSubClass == 0x42 &&
           descriptor->bInterfaceProtocol == 1;
}

static int debugging_alternate(const struct libusb_interface *interface)
{
    for (int index = 0; index < interface->num_altsetting; index++)
    {
        if (debugging_interface(&interface->altsetting[index]))
        {
            return 1;
        }
    }
    return 0;
}

static int configuration_availability(const struct libusb_config_descriptor *config)
{
    for (int index = 0; index < config->bNumInterfaces; index++)
    {
        if (debugging_alternate(&config->interface[index]))
        {
            return ACCESSORY_DEBUGGING;
        }
    }
    return ACCESSORY_AVAILABLE;
}

int accessory_availability(libusb_device *device)
{
    struct libusb_config_descriptor *config = NULL;
    if (libusb_get_active_config_descriptor(device, &config) != 0)
    {
        return ACCESSORY_UNKNOWN;
    }
    int result = configuration_availability(config);
    libusb_free_config_descriptor(config);
    return result;
}

void accessory_require_available(libusb_device_handle *handle)
{
    accessory_require(accessory_availability(libusb_get_device(handle)) == ACCESSORY_AVAILABLE,
                      "USB device reserved or unavailable; disable USB debugging and refresh discovery");
}
