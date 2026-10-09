"""T822: portable wire contracts cannot depend on native/session owners."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2] / 'android/app/src/main/java/com/blent'


class AndroidContractBoundariesTest(unittest.TestCase):
    def test_t822_wire_contracts_have_no_android_or_owner_dependencies(self):
        files = ['VideoContract.kt', 'VideoCodec.kt', 'VideoProfiles.kt',
                 'DecoderSelection.kt', 'AudioEndpoint.kt', 'CameraEndpoint.kt',
                 'VideoPacketReader.kt', 'ChannelPacketReader.kt', 'StreamFormat.kt',
                 'DecoderSelectionJson.kt']
        forbidden = r'\b(android\.|androidx\.|VideoReceiver|DecoderSession|MediaInventory|MediaProfiles)\b'
        for name in files:
            with self.subTest(file=name):
                self.assertTrue((ROOT / name).exists(), f'T822 missing independent contract: {name}')
                source = (ROOT / name).read_text()
                self.assertIsNone(re.search(forbidden, source), f'T822 native/owner dependency: {name}')


if __name__ == '__main__':
    unittest.main()
