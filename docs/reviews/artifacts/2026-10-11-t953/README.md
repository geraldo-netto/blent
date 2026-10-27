# T953: shared setup panic fixture

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Extract identical first-attempt setup failure check from audio and camera
regressions. Preserve setup/poll/destructor/cleanup panics, exact failure strings,
manual fresh Start, attempt counts, stopped state and actual shutdown completion.
All four permanent integration fixtures pass; every reviewed function now scores
<=9 (the two owner tests each previously scored 10). No production code changed.
