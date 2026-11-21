"""T1676: independent bounded packaged-runtime identity contract checks."""
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'packaging/appimage'))
import ffmpeg_identity


class IdentityTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='blent-runtime-identity-')
        self.addCleanup(temporary.cleanup)
        self.app = Path(temporary.name) / 'AppDir'
        self.root = self.app / 'usr'
        self.private = self.root / 'lib/blent-ffmpeg'
        for directory in [self.private, self.root / 'bin', self.root / 'libexec']:
            directory.mkdir(parents=True, exist_ok=True)
        (self.root / 'libexec/ffmpeg').write_bytes(b'native fixture')

    def test_t1676_declaration_lists_native_and_sorted_codec_files(self):
        for name in ['z.so', 'a.so']:
            (self.private / name).write_bytes(b'codec fixture')
        ffmpeg_identity.write(self.app)
        data = json.loads((self.root / 'bin/ffmpeg.blent-runtime.json').read_text())
        self.assertEqual(data, {'schema': 1, 'files': ['libexec/ffmpeg',
                                                     'lib/blent-ffmpeg/a.so',
                                                     'lib/blent-ffmpeg/z.so']})

    def test_t1676_count_limit_accepts_32_and_rejects_33_files(self):
        for number in range(31):
            (self.private / f'codec-{number}.so').touch()
        ffmpeg_identity.write(self.app)
        (self.private / 'overflow.so').touch()
        with self.assertRaises(ValueError):
            ffmpeg_identity.write(self.app)

    def test_t1676_escaped_json_size_remains_bounded(self):
        for number in range(31):
            (self.private / ('😀' * 55 + str(number))).touch()
        with self.assertRaises(ValueError):
            ffmpeg_identity.write(self.app)
        self.assertFalse((self.root / 'bin/ffmpeg.blent-runtime.json').exists())

    def test_t1676_paths_reject_escapes_controls_missing_and_nonregular_files(self):
        for name in ['', '/absolute', '../escape', 'lib//file', './file', 'lib/../file',
                     'lib/./file', 'lib/', 'bad\0name', 'bad\x85name', 'x' * 257, 'libexec']:
            with self.subTest(name=name), self.assertRaises(ValueError):
                ffmpeg_identity.validate(self.root, name)
        with self.assertRaises(FileNotFoundError):
            ffmpeg_identity.validate(self.root, 'missing')
        outside = self.app.parent / 'outside'
        outside.touch()
        (self.private / 'escape.so').symlink_to(outside)
        with self.assertRaises(ValueError):
            ffmpeg_identity.validate(self.root, 'lib/blent-ffmpeg/escape.so')

    def test_t1676_relative_links_within_the_runtime_remain_supported(self):
        (self.private / 'codec.so.1').touch()
        (self.private / 'codec.so').symlink_to('codec.so.1')
        ffmpeg_identity.validate(self.root, 'lib/blent-ffmpeg/codec.so')
        ffmpeg_identity.write(self.app)

    def test_t1676_component_size_accepts_512_mib_and_rejects_one_more_byte(self):
        native = self.root / 'libexec/ffmpeg'
        with native.open('wb') as stream:
            stream.truncate(536_870_912)
        ffmpeg_identity.validate(self.root, 'libexec/ffmpeg')
        with native.open('wb') as stream:
            stream.truncate(536_870_913)
        with self.assertRaises(ValueError):
            ffmpeg_identity.write(self.app)


if __name__ == '__main__':
    unittest.main()
