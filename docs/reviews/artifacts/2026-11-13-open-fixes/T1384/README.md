# T1384 — remembered approval Activity oracle

The original Activity/ordered-prelude selection fails before the correction and
passes afterward. The regression asserts that Remember is visibly on and that
approval submits persist=true. Foreground/background ownership and no-camera/
no-microphone-consent assertions remain; explicit opt-out tests are unchanged.
Production behavior is unchanged. Gradle 8.5 runs API 27/34 Robolectric fixtures;
these are not physical-device acceptance.
