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


if __name__ == '__main__':
    unittest.main()
