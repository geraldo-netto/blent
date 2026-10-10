"""T821: selection contracts must not depend back on their media consumer."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


class HostContractBoundariesTest(unittest.TestCase):
    def test_t821_selection_contract_has_no_media_or_worker_dependencies(self):
        text = (ROOT / 'host/src/selection_types.rs').read_text()
        self.assertIsNone(re.search(r'\b(crate::media|EncoderSettings|tokio|JoinHandle)\b', text),
                          'T821 selection data depends on its implementation/consumer')

    def test_t824_file_model_has_no_filesystem_extension_methods(self):
        source = (ROOT / 'common/src/storage.rs').read_text()
        self.assertNotRegex(source, r'impl\s+FileConfig\s*\{',
                            'T824 filesystem adapter adds I/O to domain values')

    def test_t824_consumers_depend_on_persistence_contracts(self):
        for name in ['gui/src/settings.rs', 'host/src/persistence.rs']:
            source = (ROOT / name).read_text().split('\n#[cfg(test)]\nmod tests')[0]
            self.assertNotIn('ConfigStore', source, 'T824 concrete storage dependency: ' + name)

    def test_t820_capture_contract_excludes_session_and_native_implementations(self):
        path = ROOT / 'host/src/capture_contract.rs'
        self.assertTrue(path.exists(), 'T820: no independent capture contract')
        source = path.read_text()
        self.assertNotRegex(source, r'\b(VideoSender|Attachment|CodecConfig|LatencyTracker|JoinHandle|CaptureManager)\b',
                            'T820: capture contract exposes an implementation')
        for name in ['host/src/capture.rs', 'host/src/capture/encoding.rs']:
            production = (ROOT / name).read_text().split('#[cfg(test)]\nmod ')[0]
            self.assertNotIn('crate::video_queue::VideoSender', production)


if __name__ == '__main__':
    unittest.main()
