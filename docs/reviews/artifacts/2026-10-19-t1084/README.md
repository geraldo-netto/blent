# T1084: auxiliary machine-power observations

PowerFactory implements the independent source factory contract using the existing
logind backend and source lifetime. Auxiliary work needs machine availability,
without compositor discovery or private DPMS helpers. This matters when composing
GUI/audio owners: those executables do not implement the daemon's private DPMS
dispatcher. No current GUI runtime use was claimed before this prerequisite.
The focused source reports logind power and correlated lock support when available,
with display support explicitly absent; auxiliary policy already ignores lock.

System-bus acquisition is shared with the existing general source without changing
its behavior. Settings validate and source identity reserves before native work;
existing admission/lifetime/backend contracts retain original tasks and cleanup.
Creation errors, absent logind and invalid settings preserve explicit capability
results. No current-executable display helper, session compositor connection or
new policy state is introduced by the focused factory.

Permanent private-bus tests exercise Wayland/X11 caller environments while an
available fake compositor exists. Only logind capabilities appear. Original
private sleep/shutdown signals produce expected machine-power transitions; these
messages affect the isolated fake service, never the host. Tests stop and join
actual backend tasks, drop the original boxed source, then await its independent
source admission certificate. Six deadline/queue boundary rejections also require
the unstarted original admission to settle. Missing logind yields explicit empty
capabilities rather than a display fallback. A deliberate faulty control delegates
to general discovery and fails on the unwanted display capability.

The first sandbox run rejected private D-Bus socket creation. The same local
fixtures pass with required sandbox access, without changing assertions or skips.
A fresh exclusively owned offline LLVM collection passes all 42 related cases;
every Rust source hash matches its manifest. Four changed production functions
pass individually: system_bus/connect_power/create 100%, general connect 96.77%.
The unchanged successful X11-probe arm in general discovery remains unmeasured;
this focused factory never uses it. Raw LCOV, object/source attestations and
fault-control evidence are retained. Source audit: 36 functions, maximum estimate 7,
cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2; native SonarQube remains T1019.
All 12 architecture checks pass. New functions retain only invocation-local source,
bus and factory borrows; original source lifetime owns admitted backend resources.
Eventual cleanup still assumes original task/backend evidence and fair retained
cleanup polling. These results do not establish unrestricted native safety or
physical suspend/resume support. Shared auxiliary composition remains T1081.
