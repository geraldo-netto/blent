"""T825: redraw must consume published inventory, never enumerate native monitors."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class MonitorInventoryBoundaryTest(unittest.TestCase):
    def test_t825_redraw_has_no_native_inventory_calls(self):
        source = (ROOT / 'gui/src/main.rs').read_text()
        redraw = source.split('fn setting_input_devices(', 1)[1].split('\n    #[cfg(not(windows))]', 1)[0]
        self.assertNotIn('NativeInventory', redraw, 'T825: native enumeration blocks redraw')
        self.assertNotIn('.snapshot()', redraw, 'T825: redraw invokes inventory provider')


if __name__ == '__main__':
    unittest.main()
