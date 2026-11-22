/* T1709/T1689: independent cadence contract, linked without a native adapter. */
#include "capture_cadence.h"
#include <assert.h>
#include <limits.h>

static void t1689_late_phase(int fps)
{
    capture_cadence_t cadence = {0};
    long long period = capture_period_ns(fps);
    long long now = 1000000000;
    capture_cadence_schedule(&cadence, now, period);
    for (int step = 0; step < 100; step++)
    {
        now = cadence.next_request_ns + step * 1234567LL;
        capture_cadence_schedule(&cadence, now, period);
        assert(cadence.next_request_ns > now);
        assert(cadence.next_request_ns <= now + period);
        assert((cadence.next_request_ns - 1000000000) % period == 0);
    }
}

static void t1689_poll_rounding(void)
{
    const long long now_ns = 1000000000;
    capture_cadence_t cadence = {0};
    const long long offsets[] = {
        -1, 0, 1, 999999, 1000000, 1000001, (long long)INT_MAX * 1000000, ((long long)INT_MAX + 1) * 1000000};
    const int expected[] = {0, 0, 1, 1, 1, 2, INT_MAX, INT_MAX};
    for (unsigned i = 0; i < sizeof(offsets) / sizeof(offsets[0]); i++)
    {
        cadence.next_request_ns = now_ns + offsets[i];
        assert(capture_request_wait_ms(&cadence, now_ns) == expected[i]);
    }
}

static void periods(void)
{
    for (int fps = 1; fps <= 240; fps++)
    {
        long long period = capture_period_ns(fps);
        assert(period * fps >= 1000000000);
        assert((period - 1) * fps < 1000000000);
        t1689_late_phase(fps);
    }
    const int invalid[] = {INT_MIN, -1, 0};
    for (unsigned i = 0; i < sizeof(invalid) / sizeof(invalid[0]); i++)
        assert(capture_period_ns(invalid[i]) == 16666667);
    assert(capture_period_ns(INT_MAX) == 1);
}

static void watchdog(void)
{
    capture_cadence_t cadence = {.last_request_ms = 1000};
    const int values[] = {INT_MIN, -1, 0, 249, 250, 500, 1000, 1001, INT_MAX};
    const int expected[] = {250, 250, 250, 250, 250, 500, 1000, 1000, 1000};
    for (unsigned i = 0; i < sizeof(values) / sizeof(values[0]); i++)
    {
        cadence.watchdog_ms = values[i];
        assert(capture_watchdog_delay(&cadence) == expected[i]);
        assert(!capture_watchdog_due(&cadence, 1000 + expected[i] - 1));
        assert(capture_watchdog_due(&cadence, 1000 + expected[i]));
        capture_cadence_backoff(&cadence);
        assert(cadence.watchdog_ms == (expected[i] < 500 ? 500 : 1000));
    }
}

static void polls(void)
{
    capture_cadence_t cadence = {.last_request_ms = 1000, .next_request_ns = 1500000000};
    const int times[] = {999, 1000, 1100, 1249, 1250, 1251, 1500};
    const int expected[] = {250, 250, 150, 1, 0, 0, 0};
    for (unsigned i = 0; i < sizeof(times) / sizeof(times[0]); i++)
        assert(capture_cadence_poll(&cadence, times[i] * 1000000LL, 1) == expected[i]);
    assert(capture_cadence_poll(&cadence, 1499000001, 0) == 1);
    assert(capture_cadence_poll(&cadence, 1500000000, 0) == 0);
    assert(!capture_fallback_due(1000, 1999));
    assert(capture_fallback_due(1000, 2000));
    assert(capture_fallback_due(1000, 2001));
}

static void publication(void)
{
    capture_publication_t publication = {.sent_us = 1000000};
    assert(capture_publication_interval_us(&publication, 60) == 200000);
    assert(capture_publication_wait_ms(&publication, 60, 1000000) == 200);
    assert(capture_publication_wait_ms(&publication, 60, 1199999) == 1);
    assert(capture_publication_wait_ms(&publication, 60, 1200000) == 1);
    publication.pending = 1;
    for (int fps = 1; fps <= 240; fps++)
    {
        long long interval = 1000000 / fps;
        assert(capture_publication_interval_us(&publication, fps) == interval);
        assert(capture_publication_wait_ms(&publication, fps, 1000000) == (interval + 999) / 1000);
        assert(capture_publication_wait_ms(&publication, fps, 1000000 + interval) == 1);
        assert(capture_publication_wait_ms(&publication, fps, 2000000) == 1);
    }
}

int main(void)
{
    periods();
    t1689_poll_rounding();
    watchdog();
    polls();
    publication();
    return 0;
}
