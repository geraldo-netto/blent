CC = gcc
CFLAGS ?= -O3
OUTPUT ?= evdi_helper
EVDI_LIBRARY_DIR ?=

# One production source list and strict language/warning policy for every package.
SOURCES = evdi_helper.c conversion.c frame_exchange.c fifo_writer.c capture.c \
          capture_activity.c capture_control.c cursor_overlay.c raw_ring.c \
          writer.c idle_control.c

# Preserve literal paths, including dollars, quotes and spaces, through Make/shell.
quote = '$(subst ','"'"',$(1))'
library_path = $(if $(value EVDI_LIBRARY_DIR),-L$(call quote,$(value EVDI_LIBRARY_DIR)))

.PHONY: all
all: $(SOURCES) $(wildcard *.h)
	mkdir -p "$$(dirname -- $(call quote,$(value OUTPUT)))"
	$(CC) -std=c11 $(CFLAGS) -Wall -Wextra -Werror $(CPPFLAGS) -I. $(SOURCES) $(LDFLAGS) $(library_path) -levdi -lpthread -Wl,-rpath,'$$ORIGIN' -o $(call quote,$(value OUTPUT))
