"""T829: generated replay must retain the decoder diagnostics implementation."""
import importlib.util
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest

ROOT = Path(__file__).resolve().parents[2]


class DecoderDiagnosticsDependencyTest(unittest.TestCase):
    def test_t829_generated_decoder_has_its_diagnostics_dependency(self):
        spec = importlib.util.spec_from_file_location('decoder_project', ROOT / 'scripts/benchmarks/decoder-project.py')
        project = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(project)
        with tempfile.TemporaryDirectory() as folder:
            output = project.prepare(SimpleNamespace(directory=Path(folder) / 'project', revision=None,
                                                     package='com.blent.decoderbench.candidate'))
            path = output / 'app/src/main/java/DecoderDiagnostics.kt'
            self.assertTrue(path.is_file(), 'T829: generated decoder cannot resolve diagnostics types')
            self.assertEqual(path.read_bytes(), (project.SOURCE / path.name).read_bytes())


if __name__ == '__main__':
    unittest.main()
