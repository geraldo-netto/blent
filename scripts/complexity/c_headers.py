"""Adapt literal header probes to the pinned C grammar without evaluating guards.

Only preprocessor header operands change; byte lengths, lines, executable tokens
and every conditional branch remain intact. The ordinary parser still rejects
malformed conditions and bodies. Macro operands already work in the grammar.
"""
import re

_COMMENT = rb'/\*[\s\S]*?\*/|//(?:\\\r?\n|[^\n])*'
_LITERAL = rb'"(?:\\[\s\S]|[^"\\])*"|\x27(?:\\[\s\S]|[^\x27\\])*\x27'
_SPACE = rb'(?:[ \t]|\\\r?\n|/\*[^\r\n]*?\*/)*'
_HEADER = rb'(?P<header>"[^"\r\n]+"|<[^>\r\n]+>)'
_PROBE = rb'\b__has_include' + _SPACE + rb'\(' + _SPACE + _HEADER + _SPACE + rb'\)'
_TOKENS = re.compile(
    rb'(?P<directive>^[ \t]*#[ \t]*(?:if|elif)\b(?:\\\r?\n|[^\n])*)|'
    + _COMMENT + rb'|' + _LITERAL, re.M)
_OPERANDS = re.compile(_COMMENT + rb'|' + _PROBE + rb'|' + _LITERAL)


def header_operands(source):
    for token in _TOKENS.finditer(source):
        if token.group('directive') is not None:
            yield from directive_operands(token)


def directive_operands(token):
    for operand in _OPERANDS.finditer(token.group()):
        if operand.group('header') is not None:
            start, end = operand.span('header')
            yield token.start() + start, token.start() + end


def parser_source(source):
    result = bytearray(source)
    for start, end in header_operands(source):
        result[start:end] = b'1' + b' ' * (end - start - 1)
    return bytes(result)
