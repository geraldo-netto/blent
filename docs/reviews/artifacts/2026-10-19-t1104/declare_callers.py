from pathlib import Path
loader=Path('/tmp/blent-t1103-evidence/scoped_rust.py').read_text().split('FILES =')[0]
namespace={};exec(compile(loader,'cached-rust-parser','exec'),namespace)
parser=namespace['parser']
def update(path, targets):
    source=path.read_bytes();tree=parser.parse(source)
    assert not tree.root_node.has_error, path
    pending=[tree.root_node];edits=[]
    while pending:
        node=pending.pop();pending.extend(node.named_children)
        if node.type!='call_expression':
            continue
        function=node.child_by_field_name('function')
        if function.text.decode() not in targets:
            continue
        arguments=node.child_by_field_name('arguments')
        assert len(arguments.named_children)==3,(path,arguments.text)
        prefix=b' ' if arguments.text[1:-1].rstrip().endswith(b',') else b', '
        edits.append((arguments.end_byte-1,prefix+b'blent_config::connection::handshake::Timeout::default()'))
    for position, addition in sorted(edits,reverse=True):
        source=source[:position]+addition+source[position:]
    path.write_bytes(source)
    print(path,len(edits))
qualified={'secure_connection::accept','secure_connection::connect'}
for name in ['host/src/accessory/setup.rs','host/src/broker/link/tests/handshake.rs','host/src/broker/linux/server/tests/tls.rs','host/src/broker/session/tests/tls.rs']:
    update(Path(name),qualified)
for name in ['host/src/secure_connection/tests.rs','host/src/secure_connection/persistence_tests.rs','host/examples/secure_connection_fixture.rs']:
    update(Path(name),{'accept','connect'})
