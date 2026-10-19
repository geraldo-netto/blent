from pathlib import Path
import re

def complete(path, names):
    source=path.read_text()
    alternatives='|'.join(re.escape(name) for name in sorted(names,key=len,reverse=True))
    pattern=re.compile(r'(?<![.\w])(?P<callee>'+alternatives+r')\((?P<arguments>[^()]*)\)')
    def replace(match):
        arguments=match['arguments'].rstrip()
        if len(arguments.rstrip(',').split(','))!=3:
            return match[0]
        separator=' ' if arguments.endswith(',') else ', '
        return match['callee']+'('+arguments+separator+'blent_config::connection::handshake::Timeout::default())'
    updated,count=pattern.subn(replace,source)
    path.write_text(updated)
    print(path,count)
for name in ['host/src/accessory/setup.rs','host/src/broker/link/tests/handshake.rs','host/src/broker/linux/server/tests/tls.rs','host/src/broker/session/tests/tls.rs']:
    complete(Path(name),{'secure_connection::accept','secure_connection::connect'})
for name in ['host/src/secure_connection/tests.rs','host/src/secure_connection/persistence_tests.rs','host/examples/secure_connection_fixture.rs']:
    complete(Path(name),{'accept','connect'})
