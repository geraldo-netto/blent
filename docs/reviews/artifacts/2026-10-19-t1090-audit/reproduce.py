"""T1090: isolated command-sequence counterexample; no actual ADB/device.
Replays Bridge.close's list/check/remove ordering against a deterministic fake.
This is supporting audit evidence, not production regression coverage or a fix.
"""
import json
from pathlib import Path
import subprocess
import tempfile

with tempfile.TemporaryDirectory(prefix='blent-t1090-') as directory:
    root=Path(directory)
    state=root/'mapping'
    state.write_text('tcp:12345')
    adb=root/'adb'
    adb.write_text("#!/usr/bin/python3\nROOT = " + repr(directory) + "\n" + """import pathlib, sys
root=pathlib.Path(ROOT)
state=root/'mapping'
args=sys.argv[1:]
assert args[:3]==['-s','original-tablet','reverse']
if args[3:]==['--list']:
    print('Usb tcp:34567 '+state.read_text())
    state.write_text('tcp:54321') # Another client rebinds after the snapshot.
elif args[3:]==['--remove','tcp:34567']:
    (root/'removed').write_text(state.read_text())
    state.unlink()
else:
    raise AssertionError(args)
""")
    adb.chmod(0o700)
    prefix=[str(adb),'-s','original-tablet','reverse']
    listing=subprocess.check_output(prefix+['--list'],text=True)
    fields=listing.split()
    assert len(fields)==3 and fields[1:] == ['tcp:34567','tcp:12345']
    assert state.read_text()=='tcp:54321'
    subprocess.run(prefix+['--remove','tcp:34567'],check=True)
    removed=(root/'removed').read_text()
    assert removed=='tcp:54321' and not state.exists()
    print(json.dumps(dict(snapshot='tcp:34567 -> tcp:12345', concurrent_rebind='tcp:34567 -> tcp:54321', removed=removed, foreign_mapping_deleted=True),indent=2))
