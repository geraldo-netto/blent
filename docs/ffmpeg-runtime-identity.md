# FFmpeg tuning-cache runtime identity

The Linux tuning cache fingerprints the selected FFmpeg program and its declared
packaged components. It rehashes file contents when opening a tuning context;
the declaration is neither a checksum cache nor an execution or trust grant.
Historical tuning still requires the current decoder and fresh render receipts.

The current packaging contract places `ffmpeg.blent-runtime.json` beside the
actual `usr/bin/ffmpeg` launcher. A symlink to that launcher uses the declaration
beside its resolved target. The filename for a differently named selected program
is its resolved filename followed by `.blent-runtime.json`.

```json
{
  "schema": 1,
  "files": [
    "libexec/ffmpeg",
    "lib/blent-ffmpeg/libx264.so.164"
  ]
}
```

The example library name is illustrative. Packaging lists the actual bundled
codec files, sorted after `libexec/ffmpeg`. Paths are relative to the launcher's
parent directory's parent (`usr` in the AppDir). This contract is independent of
the Python producer and Rust consumer implementations.

- Schema 1 has exactly `schema` and `files`; unknown fields/versions are rejected.
- The regular, non-symlink declaration is at most 16,384 bytes.
- There are 1–32 unique paths; the first is exactly `libexec/ffmpeg`.
- Each UTF-8 path is at most 256 bytes, with no control character, absolute root,
  empty segment, `.` or `..`. Components resolve inside the declared root.
- Each component must be a regular file of at most 512 MiB. In-root symlinks are
  permitted for runtime components, but not for the declaration itself.
- The launcher and every declared component are rehashed. Path boundaries and
  per-file digests are distinct inputs; changed native/codec bytes invalidate
  the tuning key even if launcher bytes and `ffmpeg -version` text are unchanged.

A directly selected native ELF executable can be fingerprinted without a
declaration. An opaque script without one, or any malformed/unreadable declared
runtime, declines the tuning cache. Capture and fresh measurement remain
available. No obsolete declaration schema or launcher-only script key is accepted.
External undeclared dynamic libraries are outside this packaged-component
identity; driver/kernel stamps and FFmpeg version output remain separate inputs.

Hashing owns one file descriptor and one 64 KiB buffer at a time. Metadata parsing
is bounded by its byte limit; accepted component lists contain at most 32 names.
Every descriptor closes before native command observation. The declaration is
not a snapshot/fence against concurrent external package mutation: packaging and
reinstallation must preserve their existing process/installation ownership rules.
