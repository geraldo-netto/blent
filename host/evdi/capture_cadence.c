#include "capture_cadence.h"
#include <limits.h>

long long capture_period_ns(int fps)
{
    long long rate = fps > 0 ? fps : 60;
    return (1000000000 + rate - 1) / rate;
}

void capture_cadence_schedule(capture_cadence_t *cadence, long long now_ns, long long period_ns)
{
    /* Skip missed deadlines; never issue catch-up requests at one observation. */
    long long remaining = period_ns;
    if (cadence->next_request_ns > 0)
        remaining -= (now_ns - cadence->next_request_ns) % period_ns;
    cadence->next_request_ns = now_ns + remaining;
}

int capture_request_wait_ms(const capture_cadence_t *cadence, long long now_ns)
{
    long long remaining = cadence->next_request_ns - now_ns;
    if (remaining <= 0)
        return 0;
    if (remaining >= (long long)INT_MAX * 1000000)
        return INT_MAX;
    /* Only the OS wait rounds up; the absolute request phase never drifts. */
    return (int)((remaining + 999999) / 1000000);
}

int capture_watchdog_delay(const capture_cadence_t *cadence)
{
    if (cadence->watchdog_ms < 250)
        return 250;
    return cadence->watchdog_ms > 1000 ? 1000 : cadence->watchdog_ms;
}

void capture_cadence_backoff(capture_cadence_t *cadence)
{
    int delay = capture_watchdog_delay(cadence) * 2;
    cadence->watchdog_ms = delay > 1000 ? 1000 : delay;
}

int capture_cadence_poll(const capture_cadence_t *cadence, long long now_ns, int update_pending)
{
    if (!update_pending)
        return capture_request_wait_ms(cadence, now_ns);
    /* A pending native update waits for its recovery deadline, never the expired
     * request phase. Ready callbacks still wake the adapter immediately. */
    int delay = capture_watchdog_delay(cadence);
    long long left = cadence->last_request_ms + delay - now_ns / 1000000;
    return left < 0 ? 0 : (left > delay ? delay : (int)left);
}

int capture_watchdog_due(const capture_cadence_t *cadence, long long now_ms)
{
    return now_ms - cadence->last_request_ms >= capture_watchdog_delay(cadence);
}

int capture_fallback_due(long long last_grab_ms, long long now_ms)
{
    return now_ms - last_grab_ms >= 1000;
}

long long capture_publication_interval_us(const capture_publication_t *publication, int fps)
{
    return publication->pending ? 1000000 / fps : 200000;
}

int capture_publication_wait_ms(const capture_publication_t *publication, int fps, long long now_us)
{
    long long interval = capture_publication_interval_us(publication, fps);
    long long remaining = publication->sent_us + interval - now_us;
    /* Keep existing full-ring policy: release wakes poll; 1 ms covers lost hints. */
    return remaining <= 0 ? 1 : (int)((remaining + 999) / 1000);
}
