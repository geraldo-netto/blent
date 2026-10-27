# T962: paused Looper retirement fixture

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

The old waiter ran only currently due Handler tasks. Native retirement's fallback
poll was scheduled five milliseconds later, leaving the request Cancelling even
after actual EXIT/death and all control jobs completed. Both API 27/34 reproduce
independently. The fixture now advances its paused clock one millisecond per tick;
the original three-second bound and all remote status/selection/unlink assertions
remain unchanged. Production code and retirement proof are unchanged.

A new permanent regression fails with the old waiter and passes with the same
clock fix. Advancing time does not retire a live native worker; confirmed silent
native death subsequently permits exactly one unlink and actual lane proof.
Supporting diagnostic reflection was temporary and is absent from the final test.
Eight remote-adapter cases pass, all reviewed functions remain <=9.

The broader 1,763-case suite has one separate native audio reader/renderer race
under T964, so no whole-suite pass is claimed yet. T963 separately tracks this
fixture's unsynchronized Binder death list. No native device, download, install
or reload. API results are Robolectric simulation.
