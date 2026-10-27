# T1146 — reject endpoints without taking a terminal

Opening an invalid camera endpoint can no longer acquire a controlling terminal. The private-session PTY regression failed before O_NOCTTY and passed afterward without touching a live terminal.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

`outputs::open_device` adds `O_NOCTTY` to its existing `O_NOFOLLOW` open flags.
A session leader can otherwise acquire a controlling terminal before a configured
PTY fails webcam-label validation. Closing that failed File does not remove the
session association. The flag prevents that effect without changing validation
or original-file lock ownership.

The permanent regression uses an owned, reaped child in a new session and a
fresh private PTY. It verifies the absence of an initial controlling terminal,
rejects the endpoint and checks that `/dev/tty` still cannot open. The original
PTY master is closed before assertions; SIGHUP handling is confined to that
single-test child. The parent observes actual child cleanup before checking its
status. The unchanged sealed regression fails before the flag fix and passes
after it. No live terminal, webcam or host session is touched.

Offline attested collection passes all 91 camera tests. All three output-policy
production functions pass individually at least 80% executable-line coverage,
including original-device admission; existing invalid paths, symlinks, regular
files, wrong labels and conflicting original locks remain covered. Scoped Clippy
and rustfmt pass. Pinned source audit: 15 functions, maximum 3; this is not native
SonarQube evidence (T1019 remains blocked). There is no new production allocation,
borrow or callback; the existing File remains stack-owned until rejection or
successful handoff, and input/path/label handling is unchanged. Raw source
manifests, profiles and attestations remain locally under
`/backups/disk2/blent-work/2026-10-19-open-items/t1146`.

</details>
