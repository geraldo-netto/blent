/* T846 test-only libusb facade; no USB runtime is linked. */
#ifndef BLENT_FAKE_LIBUSB_H
#define BLENT_FAKE_LIBUSB_H
#include <stdint.h>
#include <sys/types.h>
typedef struct libusb_device libusb_device;
typedef struct libusb_context libusb_context;
typedef struct libusb_device_handle libusb_device_handle;
#define LIBUSB_ERROR_ACCESS -3
#define LIBUSB_ERROR_NO_DEVICE -4
#define LIBUSB_TRANSFER_TYPE_MASK 3
#define LIBUSB_TRANSFER_TYPE_BULK 2
#define LIBUSB_ENDPOINT_IN 128
struct libusb_device_descriptor { uint16_t idVendor, idProduct; };
struct libusb_endpoint_descriptor { uint8_t bmAttributes, bEndpointAddress; };
struct libusb_interface_descriptor {
    uint8_t bInterfaceClass, bInterfaceSubClass, bInterfaceProtocol;
    uint8_t bNumEndpoints, bAlternateSetting, bInterfaceNumber;
    const struct libusb_endpoint_descriptor *endpoint;
};
struct libusb_interface { const struct libusb_interface_descriptor *altsetting; int num_altsetting; };
struct libusb_config_descriptor { uint8_t bNumInterfaces; const struct libusb_interface *interface; };
int libusb_init(libusb_context **context);
void libusb_exit(libusb_context *context);
uint8_t libusb_get_bus_number(libusb_device *device);
int libusb_get_port_numbers(libusb_device *device, uint8_t *ports, int count);
ssize_t libusb_get_device_list(libusb_context *context, libusb_device ***list);
void libusb_free_device_list(libusb_device **list, int unref);
int libusb_open(libusb_device *device, libusb_device_handle **handle);
void libusb_close(libusb_device_handle *handle);
libusb_device *libusb_get_device(libusb_device_handle *handle);
int libusb_get_device_descriptor(libusb_device *device, struct libusb_device_descriptor *desc);
int libusb_control_transfer(libusb_device_handle *handle, uint8_t type, uint8_t request,
    uint16_t value, uint16_t index, unsigned char *data, uint16_t length, unsigned int timeout);
int libusb_get_active_config_descriptor(libusb_device *device, struct libusb_config_descriptor **config);
void libusb_free_config_descriptor(struct libusb_config_descriptor *config);
int libusb_claim_interface(libusb_device_handle *handle, int interface);
int libusb_release_interface(libusb_device_handle *handle, int interface);
int libusb_bulk_transfer(libusb_device_handle *handle, unsigned char endpoint,
    unsigned char *data, int length, int *transferred, unsigned int timeout);
#endif
