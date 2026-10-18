"""T1074: filtered inline classes cannot borrow factory-only coverage.

XML and records belong to one bounded native report. Original compiled owner names
associate filtered inline classes with their enclosing authored methods; ambiguous
bodies fail closed. No class/file is excluded and no missing counter is invented.
"""


def filtered_origins(package):
    origins = []
    for owner in package.findall('class'):
        prefix, separator, _ = owner.attrib['name'].partition('$$inlined$')
        if separator and absent_inline_counters(owner):
            origins.append((prefix, owner.attrib['name']))
    return origins


def absent_inline_counters(owner):
    methods = owner.findall('method')
    return not methods or any(method.find("counter[@type='LINE']") is None for method in methods)


def missing_classes(method, origins):
    prefix = method['owner']+'$'+method['name'].split('$')[0]
    return [name for original, name in origins
            if original == prefix or original.startswith(prefix+'$')]


def containing(function, candidates):
    return [owner for owner in candidates if owner.file == function.file
            and owner.first <= function.first and owner.last >= function.last]


def unmeasured_bodies(functions, assigned, key):
    gaps = {}
    for function in functions:
        if function.language != 'kotlin' or function.name != '<lambda>' or assigned.get(key(function)):
            continue
        add_body_gap(function, containing(function, functions), assigned, key, gaps)
    return gaps


def add_body_gap(function, owners, assigned, key, gaps):
    for owner in owners:
        missing = [name for method in assigned.get(key(owner), [])
                   for name in method.get('unmeasured_inline_classes', [])]
        if missing:
            gap = dict(line=function.first, classes=sorted(set(missing)))
            gaps.setdefault(key(owner), []).append(gap)
            gaps.setdefault(key(function), []).append(gap)


def require_body_counters(row, gaps):
    row.update(covered=0, total=0, percent=None, passes=False, missing=[],
               evidence='missing authored inline-body counters within enclosing method; factory counters insufficient',
               unmeasured_inline_bodies=gaps)
