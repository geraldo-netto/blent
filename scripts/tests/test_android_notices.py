"""T845: Android notice assets retain canonical upstream and fork attribution."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / 'android/app/src/main/assets/licenses'


class AndroidNoticesTest(unittest.TestCase):
    def test_t845_project_assets_preserve_canonical_notices_exactly(self):
        for name in ('LICENSE', 'COPYRIGHT.md'):
            with self.subTest(name=name):
                self.assertEqual((ASSETS / name).read_bytes(), (ROOT / name).read_bytes())

    def test_t845_inventory_has_unique_coordinates_and_binary_provenance(self):
        entries = json.loads((ASSETS / 'runtime-dependencies.json').read_text())['dependencies']
        coordinates = [entry['coordinate'] for entry in entries]
        self.assertEqual(coordinates, sorted(set(coordinates)))
        self.assertIn('com.squareup.okhttp3:okhttp:4.12.0', coordinates)
        self.assertIn('com.google.guava:listenablefuture:1.0', coordinates)
        for entry in entries:
            with self.subTest(coordinate=entry['coordinate']):
                self.assertEqual(entry['license'], 'Apache-2.0')
                self.assertTrue(entry['license_evidence'].endswith(' POM'))
                for artifact in entry['artifacts']:
                    self.assertRegex(artifact['sha256'], r'^[0-9a-f]{64}$')


if __name__ == '__main__':
    unittest.main()
