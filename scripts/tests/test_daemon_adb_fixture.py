"""T1697: delayed fake ADB removal must not resurrect a retired route."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[2] / 'host/tests/daemon_startup/fake_adb.py'
SPEC = importlib.util.spec_from_file_location('daemon_fake_adb', SOURCE)
ADB = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ADB)


class DaemonAdbFixtureTest(unittest.TestCase):
    def test_t1697_delayed_removal_reads_current_routes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            route_file = root / 'routes.json'
            route_file.write_text(json.dumps({'tcp:1': 'tcp:3', 'tcp:2': 'tcp:4'}))
            hold = root / 'hold-removal'
            hold.touch()

            def other_removal(_delay):
                route_file.write_text(json.dumps({'tcp:2': 'tcp:4'}))
                hold.unlink()

            with patch.object(ADB.time, 'sleep', side_effect=other_removal):
                ADB.reverse(root, ['-s', 'TABLET', 'reverse', '--remove', 'tcp:2'])
            self.assertEqual(json.loads(route_file.read_text()), {})
