CC ?= cc
CFLAGS ?= -O2 -g
LIBUSB_CFLAGS ?= $(shell pkg-config --cflags libusb-1.0)
LIBUSB_LIBS ?= -l:libusb-1.0.so.0

blent-usb-io: stream.c accessory.c contract.c accessory.h
	$(CC) -std=c11 $(CFLAGS) -Wall -Wextra -Werror $(LIBUSB_CFLAGS) stream.c accessory.c contract.c -pthread $(LIBUSB_LIBS) -o $@

.PHONY: clean
clean:
	rm -f blent-usb-io
