/* T1610: independent product bounds, before any native admission. */
static void t1610_numeric_range(const char *name, int *target, int low, int high)
{
    for (int number = -4; number <= 260; number++)
    {
        char value[32];
        snprintf(value, sizeof(value), "%d", number);
        char *args[] = {"helper", (char *)name, value};
        *target = 42;
        helper_options_t options = parse_helper_options(3, args);
        int valid = number >= low && number <= high;
        assert(options.invalid == !valid);
        assert(*target == (valid ? number : 42));
    }
}

static void test_t1610_numeric_options(void)
{
    t1610_numeric_range("--fps", &g_capture.fps, 10, 90);
    t1610_numeric_range("--scale", &g_capture.scale, 1, 4);
    t1610_numeric_range("--conversion-threads", &g_conversion_threads, 0, 128);
    t1610_numeric_range("--card", &g_pin_card, -1, INT_MAX);
    t1610_numeric_range("--preferred-card", &g_preferred_card, -1, INT_MAX);
}

static void test_t1610_descriptor_options(void)
{
    const char *names[] = {"--capture-socket-fd", "--capture-activity-fd"};
    const char *invalid[] = {"-1", "0", "1", "2", "2147483648", "bad", "3tail", ""};
    for (size_t name = 0; name < sizeof(names) / sizeof(names[0]); name++)
    {
        for (size_t value = 0; value < sizeof(invalid) / sizeof(invalid[0]); value++)
        {
            char *args[] = {"helper", (char *)names[name], (char *)invalid[value]};
            assert(parse_helper_options(3, args).invalid);
            assert(evdi_helper_main(3, args) == 2);
        }
    }
    for (size_t name = 0; name < sizeof(names) / sizeof(names[0]); name++)
    {
        char *args[] = {"helper", "--edid", "/unused", (char *)names[name], "2147483647"};
        assert(!parse_helper_options(5, args).invalid);
        assert(evdi_helper_main(5, args) == 1);
    }
    assert(!mock_open_calls && !mock_add_calls);
    assert(g_capture.raw_ring == NULL);
    assert(g_capture.control == NULL);
}

static void test_t1610_usage(void)
{
    char *args[] = {"helper"};
    assert(evdi_helper_main(1, args) == 1);
}
