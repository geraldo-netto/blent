"""T591: exclude only Rust cfg predicates provably unavailable on Linux.

Unknown feature/target predicates remain in scope. None means either truth value
is possible; retaining such code prevents missing counters from becoming a pass.
"""
from functools import lru_cache

from inventory import rust_records


def combine(operator, values):
    if operator == 'not':
        return None if len(values) != 1 or values[0] is None else not values[0]
    decisive = operator == 'any'
    if decisive in values:
        return decisive
    return None if None in values else not decisive


def predicate(node, platform="linux"):
    if node is None:
        return None
    name = node.get('name')
    known = {'windows': platform == 'windows', 'unix': platform != 'windows'}
    if name in known:
        return known[name]
    if name in {'target_os', 'target_family'} and node.get('value') is not None:
        family = 'windows' if platform == 'windows' else 'unix'
        return node['value'] == (platform if name == 'target_os' else family)
    if name in {'all', 'any', 'not'} and node.get('args') is not None:
        return combine(name, [predicate(group, platform) for group in node['args']])
    return None


@lru_cache(maxsize=256)
def unavailable(text, platform="linux"):
    return guarded(rust_records(text + '\nmod __inventory {}')[0], platform)


def guarded(record, platform="linux"):
    return any(predicate(condition, platform) is False for condition in record.guards)
