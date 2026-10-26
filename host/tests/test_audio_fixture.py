"""T1264: native audio fixtures obey the requested ADB endpoint identity."""
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest
from unittest.mock import patch
import audio_transport
import audio_speaker_transport


class AudioFixtureTest(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix='blent-audio-fixture-')
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        (self.root/'adb').write_text(audio_transport.ADB)
        (self.root/'adb').chmod(0o700)
        self.env = dict(os.environ, BLENT_AUDIO_FIXTURE=str(self.root))

    def adb(self, *args):
        return subprocess.run([str(self.root/'adb'), '-s', 'fixture', 'reverse', *args],
                              env=self.env, capture_output=True, text=True)

    def test_t1264_reverse_preserves_both_endpoints_and_exact_removal(self):
        self.assertEqual(self.adb('--no-rebind', 'tcp:12345', 'tcp:23456').returncode, 0)
        self.assertEqual(self.adb('--list').stdout.split(), ['Usb', 'tcp:12345', 'tcp:23456'])
        self.assertNotEqual(self.adb('--remove', 'tcp:34567').returncode, 0)
        self.assertTrue((self.root/'mapping').exists())
        self.assertNotEqual(self.adb('--no-rebind', 'tcp:12345', 'tcp:34567').returncode, 0)
        self.assertEqual(self.adb('--remove', 'tcp:12345').returncode, 0)
        self.assertFalse((self.root/'mapping').exists())

    def test_t1264_tablets_connect_to_forwarded_host_port(self):
        (self.root/'mapping').write_text('Usb tcp:12345 tcp:23456')
        (self.root/'invitation').write_text('token abc port 12345 direction 2')
        for tablet in (audio_transport.tablet, audio_speaker_transport.tablet):
            failure = RuntimeError('T1264 stop before network access')
            errors = []
            with patch('socket.create_connection', side_effect=failure) as connect:
                tablet(self.root, threading.Event(), errors)
            self.assertEqual(errors, [failure])
            self.assertEqual(connect.call_args.args[0], ('127.0.0.1', 23456))


if __name__ == '__main__':
    unittest.main()
