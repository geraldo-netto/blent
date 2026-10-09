"""T810: lifecycle policy owns no Android, Compose or concrete native resources."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2] / 'android/app/src/main/java/com/blent'


class AndroidLifecycleBoundariesTest(unittest.TestCase):
    def test_t810_policy_depends_on_portable_resource_contracts(self):
        for name in ['AudioBinding.kt', 'CameraBinding.kt', 'AudioPreferences.kt', 'ResourceLifecycle.kt']:
            with self.subTest(file=name):
                self.assertTrue((ROOT / name).exists(), 'T810 lifecycle contract missing')
                text = (ROOT / name).read_text()
                self.assertIsNone(re.search(r'\b(android\.|androidx\.|CameraResources|AudioResources|Context)\b', text),
                                  f'T810 platform implementation dependency in {name}')


if __name__ == '__main__':
    unittest.main()
