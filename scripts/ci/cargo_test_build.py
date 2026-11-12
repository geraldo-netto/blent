"""T1355: derive matching production builds for explicit binary unit suites."""

VALUE_OPTIONS = {'-p', '--package', '--exclude', '--features', '-F', '--target', '--target-dir',
                 '--manifest-path', '-m', '-Z', '--profile', '-j', '--jobs', '--config', '--color', '--message-format'}
FLAGS = {'--release', '--all-features', '--no-default-features', '--workspace', '--locked', '--offline',
         '--frozen', '--keep-going', '-r', '--timings', '--ignore-rust-version', '--unit-graph', '-h', '--help', '-q', '--quiet', '-v', '--verbose', '--future-incompat-report'}
TEST_FLAGS = {'--lib', '--bins', '--tests', '--examples', '--benches', '--all-targets', '--doc',
              '--no-run', '--no-fail-fast'}
TEST_VALUES = {'--bin', '--test', '--example', '--bench'}


def short_options(token, pending):
    result = []
    for index, letter in enumerate(token[1:], 2):
        key = '-' + letter
        if key in VALUE_OPTIONS:
            value = token[index:].removeprefix('=') or next(pending)
            return [*result, key, value]
        if key not in FLAGS:
            raise ValueError(f'unsupported binary-test build option: {token}')
        result.append(key)
    return result


def option(token, pending):
    if not token.startswith('-'):
        return []
    if token.startswith('--'):
        return long_option(token, pending)
    return short_options(token, pending)


def long_option(token, pending):
    key = token.split('=', 1)[0]
    if key in VALUE_OPTIONS:
        return [token] if '=' in token else [token, next(pending)]
    if key in TEST_VALUES:
        if '=' not in token:
            next(pending)
        return []
    if token in FLAGS:
        return [token]
    if token in TEST_FLAGS:
        return []
    raise ValueError(f'unsupported binary-test build option: {token}')


def binary_build(invocation):
    if invocation[:2] != ['cargo', 'test']:
        return None
    args = invocation[2:]
    cargo = args[:args.index('--')] if '--' in args else args
    if '--bins' not in cargo:
        return None
    pending = iter(cargo)
    build = ['cargo', 'build', '--bins']
    try:
        for token in pending:
            build.extend(option(token, pending))
    except StopIteration as error:
        raise ValueError('missing binary-test build option value') from error
    return None if {'-h', '--help'} & set(build) else build
