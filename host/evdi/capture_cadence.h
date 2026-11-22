#ifndef BLENT_CAPTURE_CADENCE_H
#define BLENT_CAPTURE_CADENCE_H

/* Pure EVDI timing policy, composed by one capture event-loop owner. No native
 * resources, clocks, callbacks or retained borrows. Zero initializes an unstarted
 * phase. Clock inputs are nonnegative monotonic observations; request scheduling
 * is called only once due, with a positive period and representable next deadline.
 * Publication uses the admitted positive FPS. Native readiness and pause
 * acknowledgements remain the capture adapter's responsibility. */
typedef struct
{
    long long last_request_ms;
    long long next_request_ns;
    int watchdog_ms;
    long long last_fallback_grab_ms;
} capture_cadence_t;

typedef struct
{
    int pending;
    long long sent_us;
} capture_publication_t;

long long capture_period_ns(int fps);
void capture_cadence_schedule(capture_cadence_t *cadence, long long now_ns, long long period_ns);
int capture_request_wait_ms(const capture_cadence_t *cadence, long long now_ns);
int capture_watchdog_delay(const capture_cadence_t *cadence);
void capture_cadence_backoff(capture_cadence_t *cadence);
int capture_cadence_poll(const capture_cadence_t *cadence, long long now_ns, int update_pending);
int capture_watchdog_due(const capture_cadence_t *cadence, long long now_ms);
int capture_fallback_due(long long last_grab_ms, long long now_ms);
long long capture_publication_interval_us(const capture_publication_t *publication, int fps);
int capture_publication_wait_ms(const capture_publication_t *publication, int fps, long long now_us);

#endif
