"""T822: portable wire contracts cannot depend on native/session owners."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2] / 'android/app/src/main/java/com/blent'


class AndroidContractBoundariesTest(unittest.TestCase):
    def test_t822_wire_contracts_have_no_android_or_owner_dependencies(self):
        files = ['VideoContract.kt', 'VideoCodec.kt', 'VideoProfiles.kt',
                 'DecoderSelection.kt', 'AudioEndpoint.kt', 'CameraEndpoint.kt',
                 'VideoPacketReader.kt', 'StreamFormat.kt',
                 'DecoderSelectionJson.kt']
        forbidden = r'\b(android\.|androidx\.|VideoReceiver|DecoderSession|MediaInventory|MediaProfiles)\b'
        paths = [ROOT / name for name in files]
        paths.append(Path(__file__).resolve().parents[2] /
                     'scripts/benchmarks/android-decoder/input/ChannelPacketReader.kt')
        for path in paths:
            with self.subTest(file=path.name):
                self.assertTrue(path.exists(), f'T822 missing independent contract: {path.name}')
                source = path.read_text()
                self.assertIsNone(re.search(forbidden, source), f'T822 native/owner dependency: {path.name}')

    def test_t1750_input_contracts_are_independent_of_native_ownership(self):
        forbidden = r'\b(android\.|androidx\.|MediaCodec|DecoderSession|CodecLifetime|CodecInputAccess)\b'
        for name in ('DecoderInputPacket.kt', 'DecoderInputAccess.kt', 'DecoderOutputPolicy.kt'):
            with self.subTest(file=name):
                self.assertIsNone(re.search(forbidden, (ROOT / name).read_text()))


if __name__ == '__main__':
    unittest.main()
