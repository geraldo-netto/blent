# T1213: explicit native USB build compiler and freshness

Permanent isolated controls fail against the original Makefile for an absent
helper with the root-selected gcc, and for newer stale outputs with default and
explicit compilers. Recursive Make now receives the selected compiler explicitly;
production source/release/CI/Arch workflows force the tiny native helper build
from current C inputs. Current callers keep one native helper output owner; no
Cargo-generated USB I/O binary or old artifact fallback. All8 Make controls,
6 package fixtures and the Arch build contract pass; the changed Arch build
function measures100% native Bash executable lines with original source/origin
attestation. Raw red/green/trace/source evidence retained. Other changed build
lines are module-level Make/Shell wiring, not new production functions.

No native implementation, device operation, deployment, EVDI build, package
transfer or upload. Generated output belongs to the caller's build directory;
compiler/make run in its existing foreground group and failure propagates before
installation/staging. Existing installer publication/cleanup remains unchanged.
Local source estimates <=9; native Sonar availability remains T1019. T1210 owns
remaining current source-output fixtures separately.
