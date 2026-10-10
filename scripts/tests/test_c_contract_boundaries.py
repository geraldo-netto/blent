"""T823: public pixel contracts cannot expose worker or exchange ownership layout."""
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class CContractBoundariesTest(unittest.TestCase):
    def test_t823_public_headers_hide_native_owner_layout(self):
        for name in ('conversion.h', 'frame_exchange.h'):
            source = (ROOT / 'host/evdi' / name).read_text()
            self.assertNotIn('pthread', source, name + ' exposes thread/lock layout')
            self.assertNotIn('_private.h', source, name + ' reexports private owner layout')
        for name in ('capture.c', 'writer.c'):
            source = (ROOT / 'host/evdi' / name).read_text()
            self.assertNotIn('->frames->', source, name + ' reaches into exchange ownership')

    def test_t823_public_pixel_contracts_compile_independently(self):
        source = b'#include "conversion.h"\n#include "frame_exchange.h"\nvoid consume(conv_pool_t *p, const conv_job_t *j, const frame_lease_t *l) { (void)l; conv_pool_convert(p,j); }\n'
        result = subprocess.run(['cc', '-std=c11', '-Werror', '-Ihost/evdi', '-x', 'c', '-fsyntax-only', '-'], input=source, cwd=ROOT, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr.decode())


if __name__ == '__main__':
    unittest.main()
