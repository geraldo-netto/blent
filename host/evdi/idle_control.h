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

#endif
