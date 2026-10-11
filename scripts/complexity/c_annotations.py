"""T844: blank only proven object-like attribute macros in declaration types.

The pinned parser mistakes an annotation for the return type. Inspect its partial
syntax tree, never rewrite executable bodies, and preserve every byte offset.
A macro is eligible only if all its definitions contain attributes or nothing.
Unknown, function-like, undefined or conflicting macros are left to fail closed.
"""


def nodes(node):
    yield node
    for child in node.named_children:
        yield from nodes(child)


def attribute_value(parser, value):
    if not value.strip():
        return True
    root = parser.parse(value + b' void blent_annotation(void) {}').root_node
    if root.has_error or len(root.named_children) != 1:
        return False
    function = root.named_children[0]
    if function.type != 'function_definition':
        return False
    prefix = [child for child in function.named_children if child.end_byte <= len(value)]
    return bool(prefix) and all(child.type == 'attribute_specifier' for child in prefix)


def annotation_names(parser, root):
    allowed, denied = set(), set()
    for node in nodes(root):
        if node.type not in {'preproc_def', 'preproc_function_def'}:
            continue
        name = node.child_by_field_name('name').text
        value = node.child_by_field_name('value')
        if node.type == 'preproc_def' and attribute_value(parser, value.text if value else b''):
            allowed.add(name)
        else:
            denied.add(name)
    return allowed - denied - undefined_names(root)


def undefined_names(root):
    names = set()
    for node in nodes(root):
        if node.type != 'preproc_call':
            continue
        if node.child_by_field_name('directive').text != b'#undef':
            continue
        argument = node.child_by_field_name('argument')
        if argument is not None:
            import re
            name = re.match(rb'[A-Za-z_][A-Za-z_0-9]*', argument.text.lstrip())
            if name is not None:
                names.add(name.group())
    return names


def declaration_types(node):
    if node.type in {'compound_statement', 'attribute_specifier'}:
        return
    if node.type in {'function_definition', 'declaration'}:
        value = node.child_by_field_name('type')
        if value is not None and value.type == 'type_identifier':
            yield value
    for child in node.named_children:
        yield from declaration_types(child)


def parser_source(source, parser):
    root = parser.parse(source).root_node
    names = annotation_names(parser, root)
    result = bytearray(source)
    for node in declaration_types(root):
        if node.text in names:
            result[node.start_byte:node.end_byte] = b' ' * (node.end_byte - node.start_byte)
    return bytes(result)
