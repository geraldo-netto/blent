CC ?= cc
CFLAGS ?= -O2 -g
LIBUSB_CFLAGS ?= $(shell pkg-config --cflags libusb-1.0)
LIBUSB_LIBS ?= -l:libusb-1.0.so.0

OUTPUT ?= blent-usb-io

$(OUTPUT): stream.c accessory.c contract.c discovery.c accessory.h
	mkdir -p "$(dir $(OUTPUT))"
	$(CC) -std=c11 $(CFLAGS) -Wall -Wextra -Werror $(LIBUSB_CFLAGS) stream.c accessory.c contract.c discovery.c -pthread $(LIBUSB_LIBS) -o "$@"

.PHONY: clean
clean:
	rm -f "$(OUTPUT)"
