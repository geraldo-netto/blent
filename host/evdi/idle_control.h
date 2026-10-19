#ifndef BLENT_IDLE_CONTROL_H
#define BLENT_IDLE_CONTROL_H

/* Linux advisory lease, read afresh from a bounded (160-byte) private regular
 * file. Five unsigned decimal fields: source device, inode, owner cookie,
 * CLOCK_MONOTONIC expiry in ms, cadence in ms. Only 500/1000 ms and expiry
 * strictly after now, at most 2000 ms ahead, are accepted. Zero means no advice;
 * each adapter retains its own compatibility fallback.
 *
 * Caller owns and pins the ORIGINAL source descriptor through the read and all
 * uses of the result; a pathname or reused descriptor is not source authority.
 * Cookie lifetime and old-source retirement are the publishing owner's duty.
 * No request authorizes source replacement or native release. Result is an
 * instantaneous observation: adapters must recheck within 200 ms, wake for
 * fresh data/stop, and retain actual frame leases through every native access.
 *
 * path is an optional borrowed NUL-terminated name; source is a borrowed open
 * descriptor; now_ms is a representable nonnegative monotonic time. This call
 * retains neither borrow, closes its own opened file on every path, and uses
 * only bounded stack storage. No global state or allocations. */
int idle_control_interval(const char *path, int source, long long now_ms);

/* One native reader owns at most one O_PATH descriptor for advice present
 * before its first source access. That ORIGINAL object is never admissible for
 * this reader, even if a source inode is reused after publisher loss. Current
 * publishers replace records atomically; they never rewrite a record in place.
 *
 * Initialize before begin; zero initialization is disabled and owns no fd.
 * begin on a live reader rejects without disturbing its original ownership.
 * Missing advice admits future publications. Any other admission failure keeps
 * this reader compatible; no file is removed. Borrow this value only within its
 * owning native worker. end follows its last source/control access and precedes
 * return/join; it is idempotent. Logical expiry/clear never releases the pin.
 * No callbacks, allocations or other retained resources. Admission accepts only
 * a private regular file of at most 159 bytes; unknown objects disable advice. */
typedef struct
{
    int fd;
    int admitted;
} idle_control_reader_t;

#define IDLE_CONTROL_READER_INITIALIZER                                                            \
    {                                                                                              \
        .fd = -1, .admitted = 0                                                                    \
    }
int idle_control_begin(idle_control_reader_t *reader, const char *path);
int idle_control_reader_interval(const idle_control_reader_t *reader, const char *path, int source,
                                 long long now_ms);
void idle_control_end(idle_control_reader_t *reader);

#endif
