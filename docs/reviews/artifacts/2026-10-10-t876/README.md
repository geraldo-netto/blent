# T876 read-only supervised USB inventory

The isolated helper accepts --list LIMIT (default selected by host:64, maximum512)
and emits a 16-byte BLUI v1 header plus fixed 16-byte candidate records. Enumeration
uses stock libusb inventory/descriptors only; it never opens devices, queries
strings, claims interfaces or changes accessory mode. Root controllers/hubs are
excluded. Capacity overflow and native errors fail explicitly. Observed accessory
VID/PID indicates a candidate only, never identity or approval.

Rust Scan owns a supervised helper, enforces one deadline across bounded output,
exit and retirement attempt, parses exact records and rejects duplicate physical
paths/malformed extents. Completion remains independently observable after error
or cancellation. Timeout does not prove native retirement. T853 owns opaque target
IDs, selection validation and process/client composition.

Offline validation:14 accessory Rust tests (seven new),15 native Python tests
(four new discovery plus retained AOA/stream cases and dispatch) pass. Native
fixtures run ASan+UBSan and reject prohibited USB calls. All nine Rust production
functions pass per-function80% LLVM coverage; all11 C functions across discovery.c
and stream.c have100% executable-line GCC coverage. All83 reviewed touched
production/test functions have complexity at most9; retained stream scenarios
were split into cohesive helpers without weakening assertions. Source hashes,
LLVM/GCC counters, logs and complexity evidence are retained.

The real helper also builds against cached installed libusb headers and the
stock runtime library. It was not executed on live USB devices. No downloads,
application installations or USB mode changes; production discovery integration
remains T853 and production media acceptance remains T855/T857 onward.
