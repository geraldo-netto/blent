/* T762: malformed explicit card selection must never select card zero. */
static void test_t762_arguments(void) {
    const char *invalid[] = {"", "junk", "1tail", "2147483648", "-2147483649",
        "99999999999999999999999999999", "-2"};
    for (unsigned i = 0; i < sizeof(invalid) / sizeof(invalid[0]); i++) {
        g_pin_card = 7;
        assert(set_numeric_option("--card", invalid[i]) == -1);
        assert(g_pin_card == 7);
        assert(set_numeric_option("--preferred-card", invalid[i]) == -1);
    }
    assert(set_numeric_option("--card", "-1") == 1 && g_pin_card == -1);
    assert(set_numeric_option("--card", "2147483647") == 1 && g_pin_card == INT_MAX);
    assert(set_numeric_option("--fps", "60junk") == -1);
    assert(set_numeric_option("--scale", "junk") == -1);
    assert(set_numeric_option("--unknown", "junk") == 0);
    char *unknown[] = {"helper", "--unknown", "x"};
    char *missing[] = {"helper", "--card"};
    assert(evdi_helper_main(3, unknown) == 2);
    assert(evdi_helper_main(2, missing) == 2);
}
