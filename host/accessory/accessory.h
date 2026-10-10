/* T846 Linux USB adapter for isolated helper processes; no application authority. */
#ifndef BLENT_AOA_PROBE_H
#define BLENT_AOA_PROBE_H
#include <stdint.h>
typedef struct libusb_context libusb_context;
typedef struct libusb_device_handle libusb_device_handle;
typedef struct { uint8_t bus, ports[7]; int count; } accessory_target;
typedef struct { libusb_context *context; libusb_device_handle *handle; int interface, claimed;
                 unsigned char input, output; uint16_t output_packet; } accessory_usb;
accessory_target accessory_parse_target(const char *bus, const char *ports);
int accessory_lock(accessory_target target);
void accessory_open(accessory_usb *usb, accessory_target target);
void accessory_transfer(accessory_usb *usb, int receive, unsigned char *data, int length);
void accessory_close(accessory_usb *usb);
void accessory_require(int condition, const char *message);
uint64_t accessory_clock(void);
#endif
