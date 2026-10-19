"""T1087: portable and native common builds keep private feature composition clean."""
from pathlib import Path
import os
import tempfile
import unittest

import test_secure_connection_interop as interop


class CommonFeatureBuildTest(unittest.TestCase):
    def compile(self, features):
        # Existing command owner retains its exact child/group and joins on
        # cancellation. The log is funded until that original command settles.
        with tempfile.TemporaryDirectory(prefix='blent-t1087-') as temporary:
            interop.SecureConnectionInteropTest().command([
                'cargo', 'rustc', '--offline', '--locked', '-p', 'blent-config',
                '--lib', *features, '--', '-D', 'dead-code',
            ], dict(os.environ, CARGO_NET_OFFLINE='true'), Path(temporary) / 'compile.log')

    def test_t1087_portable_build_has_no_unused_native_batch(self):
        self.compile(['--no-default-features'])

    def test_t1087_native_default_keeps_batch_contract(self):
        self.compile([])


if __name__ == '__main__':
    unittest.main()
