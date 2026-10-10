/* T846 isolated, synthetic native probe. Not a production transport API. */
#ifndef BLENT_AOA_PROBE_H
#define BLENT_AOA_PROBE_H
#include <stdint.h>
typedef struct libusb_context libusb_context;
typedef struct libusb_device_handle libusb_device_handle;
typedef struct { uint8_t bus, ports[7]; int count; } probe_target;
typedef struct { libusb_context *context; libusb_device_handle *handle; int interface, claimed;
                 unsigned char input, output; } probe_usb;
probe_target probe_parse_target(const char *bus, const char *ports);
void probe_open(probe_usb *usb, probe_target target);
void probe_transfer(probe_usb *usb, int receive, unsigned char *data, int length);
void probe_close(probe_usb *usb);
void probe_require(int condition, const char *message);
uint64_t probe_clock(void);
#endif
