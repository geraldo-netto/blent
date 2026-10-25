"""T1264: temporary diagnostic fixture correction; repository files untouched."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[4]
spec = importlib.util.spec_from_file_location('offline', ROOT/'scripts/ci/test-host-offline.py')
offline = importlib.util.module_from_spec(spec)
spec.loader.exec_module(offline)
target = Path('/backups/disk2/blent-work/2026-10-20-all-open/T835/target')
sdk = Path('/home/netto/.local/share/flatpak/runtime/org.freedesktop.Sdk/x86_64/25.08/active/files').resolve()
codecs = Path('/home/netto/.local/share/flatpak/runtime/org.freedesktop.Platform.codecs-extra/x86_64/25.08-extra/active/files').resolve()
env = offline.environment(sdk, codecs, target, Path('/usr/lib/llvm-18/lib'), os.environ)
env['PATH'] = offline.native_tools(target, env.get('PATH', os.defpath))
with tempfile.TemporaryDirectory(prefix='blent-review-audio-') as directory:
    private = Path(directory)
    for name in ('audio_native.py','audio_speakers.py','audio_transport.py','audio_speaker_transport.py'):
        source = (ROOT/'host/tests'/name).read_text()
        source = source.replace("(root/'mapping').write_text('Usb tcp:'+port+' tcp:'+port)", "(root/'mapping').write_text('Usb '+args[-2]+' '+args[-1])")
        source = source.replace("port = int(words[words.index('port') + 1])", "port = int((root/'mapping').read_text().split()[-1].split(':')[1])")
        (private/name).write_text(source)
    for name in ('audio_native.py','audio_speakers.py'):
        print('Temporary corrected fixture:', name, flush=True)
        result = subprocess.run(['python3',str(private/name),str(target/'debug/blent-audio'),str(target/'debug/blent')],env=env)
        print('Exit:',result.returncode,flush=True)
        if result.returncode:
            raise SystemExit(result.returncode)
