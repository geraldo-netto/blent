from pathlib import Path
import importlib.util,contextlib,io,json,tempfile
from unittest.mock import patch
root=Path(__file__).resolve().parents[4]
spec=importlib.util.spec_from_file_location('portability',root/'scripts/ci/verify-portability.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
for unportable in ['blent-usb-broker','blent-usb-io']:
    checked=[]
    def output(command, **kwargs):
        if command[0]=='readelf': return '(RUNPATH) [$ORIGIN]\n(NEEDED) [libevdi.so.1]'
        name=Path(command[-1]).name; checked.append(name)
        return 'GLIBC_2.40' if name==unportable else 'GLIBC_2.36'
    with tempfile.TemporaryDirectory() as directory:
        folder=Path(directory)
        for name in ['blent','blent-gui','blent-audio','blent-usb-broker','blent-usb-io','evdi_helper','libevdi.so.1.15.0']:
            (folder/name).touch()
        (folder/'libevdi.so.1').symlink_to('libevdi.so.1.15.0')
        with patch.object(module.subprocess,'check_output',side_effect=output), contextlib.redirect_stdout(io.StringIO()):
            module.verify_bundle(folder)
        print(json.dumps({'unsupported_shipped_binary':unportable,'required_glibc':'2.40','bundle_accepted':True,'inspected':checked}))
