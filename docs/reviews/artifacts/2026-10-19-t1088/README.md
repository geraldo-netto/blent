# T1088: successful camera work cannot hide cleanup failure

run_native preserves the original connection cleanup Result. A successful serve
now returns cleanup failure; an already failed serve remains the primary error.
The existing cleanup diagnostic remains, including when both operations fail.
No resource ownership or native retirement certificate was added by this fix;
retained camera/legacy mapping obligations remain explicit T1089/T1090 work.

The permanent normal-suite regression runs in a private mount namespace with
fresh owned PTYs, private labels and a fake ADB executable. No live camera/tablet
or host mount changes occur. A stopped session serves successfully, but fake
original mapping cleanup fails. Before the fix it incorrectly returns success;
afterward the same test returns the original cleanup diagnostic. The fake mapping
file is removed and both device leases verified before the result assertion,
including against faulty behavior. Existing private-device lifecycle/label/lock
regressions remain and pass. A common fixture runner retains both PTY masters and
temporary namespace data until each native action completes; its exact child is
waited before parent return. Production allocation/admission/lifetime bounds are
unchanged; the additional local Result owns only its existing error payload.

Both private native cases pass in a fresh exclusively owned offline LLVM target.
The complete changed production function run_native measures 100% individually.
Raw LCOV, source/object attestations and failing-before/passing-after logs remain.
All 12 architecture checks pass. Source estimates: 20 functions, maximum 5,
repository counter and pinned cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2;
native SonarQube remains unavailable under T1019. No physical acceptance or full
camera cleanup fix is claimed. No network transfer or installation occurred.
