"""T827: deliberately scoped dependency rules, not a general Rust/C parser."""
import re

# A contract may name portable values but never these concrete owners/adapters.
FORBIDDEN = {
    'host/src/selection_types.rs': {'media', 'media_storage', 'capture', 'worker', 'trial', 'health', 'EncoderSettings', 'tokio', 'JoinHandle'},
    'host/src/media.rs': {'capture', 'session', 'attachment', 'selection_worker', 'CaptureManager', 'VideoSender'},
    'host/src/capture_contract.rs': {'capture', 'attachment', 'session', 'VideoSender', 'Attachment', 'CodecConfig', 'LatencyTracker', 'JoinHandle', 'CaptureManager'},
    'common/src/persistence_contract.rs': {'storage', 'ConfigStore', 'Path', 'PathBuf', 'File', 'Mutex', 'RwLock'},
    'host/src/persistence.rs': {'ConfigStore'},
    'gui/src/settings.rs': {'ConfigStore'},
    'host/src/capture.rs': {'VideoSender'},
    'host/src/capture/encoding.rs': {'VideoSender'},
}
C_OWNERS = {'conversion_private.h': {'conversion.c', 'evdi_helper.c'},
            'frame_exchange_private.h': {'frame_exchange.c', 'evdi_helper.c'}}


def uncomment(source):
    # Strings remain: C include names and Rust include! must stay visible.
    # Consume nested Rust block comments, preserving newlines and token separation.
    source = re.sub(r'//[^\n]*', ' ', source)
    depth, result, index = 0, [], 0
    while index < len(source):
        pair = source[index:index + 2]
        if pair == '/*':
            depth += 1
            result.append(' ')
            index += 2
        elif pair == '*/' and depth:
            depth -= 1
            index += 2
        else:
            if not depth or source[index] == '\n':
                result.append(source[index])
            index += 1
    return ''.join(result)


def violations(name, source):
    code = uncomment(source)
    # Consumer fixtures can use concrete implementations; production cannot.
    if name in {'gui/src/settings.rs', 'host/src/persistence.rs', 'host/src/capture.rs', 'host/src/capture/encoding.rs'}:
        code = re.split(r'#\[cfg\(test\)\]\s*mod\s', code)[0]
    tokens = set(re.findall(r'\b[A-Za-z_]\w*\b', code))
    errors = []
    for symbol in sorted(tokens & FORBIDDEN.get(name, set())):
        edge = rf'(?:\b{symbol}\s*::|::\s*{symbol}\b|\buse\s+[^;]*\b{symbol}\b)'
        if symbol[0].isupper() or re.search(edge, code):
            errors.append(symbol)
    if name in FORBIDDEN:
        if re.search(r'\b(?:include|include_str|include_bytes)\s*!', code):
            errors.append('unreviewed source inclusion')
    if name.startswith('host/evdi/'):
        errors.extend(c_violations(name.rsplit('/', 1)[1], code))
    return errors


def c_violations(base, code):
    errors = []
    for header, owners in C_OWNERS.items():
        if re.search(r'#\s*include\s*[<"]' + header + r'[>"]', code) and base not in owners:
            errors.append('private owner include: ' + header)
    if base in {'conversion.h', 'frame_exchange.h'}:
        if re.search(r'\b(?:pthread_\w+|struct\s+(?:conv_pool|frame_exchange)\s*\{)', code):
            errors.append('public native layout')
    if base in {'capture.c', 'writer.c'} and re.search(r'->\s*frames\s*->', code):
        errors.append('exchange layout access')
    return errors
