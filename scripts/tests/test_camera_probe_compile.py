"""T1106: keep the opt-in camera probe composed without running native capture."""
from pathlib import Path
import os
import tempfile
import unittest

import test_secure_connection_interop as interop


class CameraProbeCompileTest(unittest.TestCase):
    def test_t1106_camera_feedback_probe_compiles_with_shared_command_adapter(self):
        # Reuse T1079's original-process/group join and cancellation sponsor.
        # Its log remains owned until the compiler exits; no elapsed kill cap.
        with tempfile.TemporaryDirectory(prefix='blent-t1106-compile-') as temporary:
            interop.SecureConnectionInteropTest().command(
                ['cargo', 'check', '--offline', '--locked', '-p', 'blent',
                 '--features', 'accessory-transport', '--example', 'camera-feedback-probe'],
                dict(os.environ, CARGO_NET_OFFLINE='true'), Path(temporary) / 'compiler.log')
