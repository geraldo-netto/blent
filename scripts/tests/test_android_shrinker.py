"""T1753: release shrinking must not retain whole application/library packages."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


class AndroidShrinkerTest(unittest.TestCase):
    def test_t1753_no_blanket_keeps_or_warning_suppression(self):
        rules = (ROOT / 'android/app/proguard-rules.pro').read_text()
        active = '\n'.join(line.split('#', 1)[0] for line in rules.splitlines())
        for package in ('com.blent', 'okhttp3', 'okio', 'org.json'):
            with self.subTest(package=package):
                self.assertNotRegex(active, r'-(?:keep|dontwarn)\b[^\n]*' + re.escape(package) + r'\.\*\*')
        self.assertNotRegex(active, r'-(?:dontshrink|dontoptimize|dontobfuscate)\b')

    def test_t1753_release_uses_android_entrypoint_rules(self):
        build = (ROOT / 'android/app/build.gradle.kts').read_text()
        self.assertIn('isMinifyEnabled = true', build)
        self.assertIn('isShrinkResources = true', build)
        self.assertIn('getDefaultProguardFile("proguard-android-optimize.txt")', build)


if __name__ == '__main__':
    unittest.main()
