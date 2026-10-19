from pathlib import Path
import gzip
import os
import sys
import xml.etree.ElementTree as ET
sys.path.insert(0,'scripts/tests')
import test_secure_connection_interop as interop
root=Path.cwd().resolve();base=Path('/tmp/blent-t898-evidence')
fixture=root/'testdata/connection-settings-v1.tsv';original=fixture.read_bytes()
(base/'correct-fixture.tsv').write_bytes(original)
needle=b'trusted_peers\t--trust-peers\t64\t1\t1024';assert original.count(needle)==1
runner=interop.SecureConnectionInteropTest()
def rejected(arguments, logfile):
    try:
        runner.command(arguments,dict(os.environ,CARGO_NET_OFFLINE='true'),logfile)
    except AssertionError:
        return
    raise AssertionError('T898 deliberately wrong fixture was accepted')
try:
    fixture.write_bytes(original.replace(needle,b'trusted_peers\t--trust-peers\t63\t1\t1024'))
    rejected(['cargo','test','--offline','--locked','-p','blent','--features','accessory-transport','--lib','t898_shared_defaults_and_boundaries_reach_validated_cli_snapshot','--','--nocapture'],base/'fault-rust.log')
    text=(base/'fault-rust.log').read_text();assert 'left: 64' in text and 'right: 63' in text
    print('Rust rejected wrong shared default',flush=True)
    rejected([str(root/'android/gradlew'),'-p',str(root/'android'),'--offline','--no-daemon','--max-workers=2','-I',str(root/'scripts/coverage/android.init.gradle'),':app:testDebugUnitTest','--tests','com.blent.connection.ConnectionSettingsContractTest'],base/'fault-android.log')
    report=root/'android/app/build/test-results/testDebugUnitTest/TEST-com.blent.connection.ConnectionSettingsContractTest.xml'
    xml=ET.parse(report).getroot();failures=[failure.attrib.get('message','') for failure in xml.findall('.//failure')]
    assert any('expected:<63> but was:<64>' in message.replace('[','').replace(']','') for message in failures),failures
    (base/'fault-android.xml.gz').write_bytes(gzip.compress(report.read_bytes(),mtime=0))
    print('Android rejected wrong shared default',flush=True)
finally:
    fixture.write_bytes(original)
