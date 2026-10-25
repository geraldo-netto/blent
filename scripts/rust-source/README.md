# Rust source frontend (T1221)

This development tool parses one UTF-8 Rust compilation unit from stdin with
`syn` 2.0.119 and emits one JSON document on stdout. Python metric, coverage and
mutation inventories consume this contract through `complexity/rust_source.py`.
It replaces the previous Rust tree-sitter grammar; there is no fallback parser.
Parse failure has a nonzero exit and a diagnostic on stderr, never partial rows.
The parser does not type-check, expand macros or evaluate conditional compilation.

The independent output contract is `{ "version": 1, "records": [...] }`.
Records occur in source traversal order. Each record has these fields:

| Field | Contract |
| --- | --- |
| `kind` | `function`, `module` or literal-path `include` |
| `name` | Authored function/module identifier, or decoded include path |
| `first`, `last` | One-based inclusive source lines |
| `start`, `end` | UTF-8 byte offsets, start inclusive and end exclusive |
| `guards` | Local and enclosing `cfg` predicates, without evaluating features |
| `test_only` | Local/enclosing test attribute, `cfg(test)` or leading `cfg(all(test, ...))` |
| `detail` | Function: `{ "score": integer }`; module: `{ "inline": boolean, "path": string-or-null }`; include: null |

A predicate has `name` (string or null), optionally `value` (string or null)
or `args` (array of predicates or null). Unrecognized metadata remains unknown.
Platform filtering excludes only predicates provably false; unknown features
remain visible. Function spans include visibility/signature/body and exclude
attributes and preceding comments. Declarations without bodies are not functions.
Module paths retain the existing file-resolution policy in the Python inventory.
Only literal `include!` paths create include edges; other macro tokens stay opaque.

`score.rs` implements the metric independently of native coverage. Nonempty
functions, closures, if/loop/while/for, logical operators and nonempty match arms
increment scores. Nested function/method bodies contribute to their container
and also receive their own row. Existing 9/10 and macro-opacity oracles remain.
`owners.rs` owns source spans and attribute ancestry, without deriving ownership
from a score or a native line counter.

The adapter builds only from the locked local Cargo cache, waits for compilation,
and reuses a binary keyed by all source/manifest/lock bytes. Each parser process
owns one input, AST and output; it exits and is reaped before results are used.
There are no callbacks, device handles, unsafe blocks or borrowed views escaping
the process. Rust owns AST/string/vector allocations; lexical return or error
drops them. Output records own copied values. Python retains at most 32 parsed
inputs/results and one binary path per process; callers treat cached rows and
their metadata as read-only. Memory/output scale with the finite input AST and
its enclosing attribute lists. This development tool is outside application
coverage scope; permanent regressions still run in the normal metric/inventory
suites. Hosted parser tests do not establish native application/device safety.

Run the normal tests from the repository root with the installed Python C/Shell
parser dependencies and JDK:

```sh
python3 -m unittest discover -s scripts/complexity -p 'test_*.py'
python3 -m unittest discover -s scripts/coverage -p 'test_*.py'
python3 -m unittest discover -s scripts/mutation -p test_scope.py
cargo fmt --manifest-path scripts/rust-source/Cargo.toml --check
```
