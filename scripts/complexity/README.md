# Cyclomatic complexity gate (T381)

From the repository root, using Python 3.12 and JDK 17 (Python 3.12/JDK 21
also work locally):

```sh
python3 -m venv /tmp/blent-complexity-venv
/tmp/blent-complexity-venv/bin/pip install -r scripts/complexity/requirements.txt
/tmp/blent-complexity-venv/bin/python -m unittest discover -s scripts/complexity -p 'test_*.py'
/tmp/blent-complexity-venv/bin/python scripts/complexity/check.py
```

`--verbose` prints every function's source location, name and score. Exit 1
means a score exceeds 9; exit 2 means the audit could not complete. Parse errors
are failures, never permission to skip a file. The manually dispatched CI workflow runs the boundary/rule tests
and the whole-project gate. Pushes and pull requests do not trigger CI.

Python parser dependencies are pinned in `requirements.txt`. Rust uses the
locked `syn` 2.0.119 frontend in `../rust-source`; its first invocation builds
with `cargo build --offline --locked` into `$XDG_CACHE_HOME/blent-rust-source`
(default `~/.cache/blent-rust-source`). Populate the Cargo cache explicitly
before an offline run if dependencies are absent; the checker never downloads
them. Rust source, manifest and lock hashes key the executable cache. See the
[source-record contract](../rust-source/README.md). Kotlin uses the
1.9.20 compiler's PSI parser, with pinned SHA-256 checksums for its Maven jars
in `kotlin.py`. The first run downloads those jars from Maven Central into
`$XDG_CACHE_HOME/blent-complexity` (default `~/.cache/blent-complexity`).
Subsequent runs reuse verified jars and a checker jar keyed by source hash.
No Sonar implementation is vendored or required at runtime.

Owner clarification2026-10-21 accepts local cyclomatic tools as sufficient gate
evidence; a SonarQube installation or vendor certification is not required.
This checker remains a source analysis with the rules and limitations below.
Keep per-function results, tool versions, source hashes and supported scope;
parse failures or unavailable measurements still cannot pass. T1244's scoped
verification is recorded in
[the local analysis evidence](../../docs/reviews/artifacts/2026-10-21-local-complexity/T1244/README.md).

## Scope

Git-tracked files and untracked, nonignored files are included, including tests
and this checker. Deleted files, dependency/build/cache directories, generated
Gradle wrappers and the upstream `host/evdi/evdi_lib.h` are excluded explicitly.
Kotlin build scripts, Arch `PKGBUILD`/`.install`, Debian `postinst` and Python
heredocs passed directly to `python`/`python3`
are included. Embedded Python retains the containing script's line numbers.

Only functions/methods are scored: top-level scripts and Make recipes are not
implicitly treated as functions. Source
strings containing fixtures or generated code are not analyzed as code.

## Pinned rules and limitations

This is a source audit aligned with the following Sonar cyclomatic rules,
**not a native SonarQube run or cognitive-complexity score**. Recheck rules and
fixtures deliberately when updating a parser or Sonar reference.

| Language | Count and reference |
| --- | --- |
| Rust | Nonempty function bodies, closures, if/loop/while/for, logical operators and nonempty match arms. Nested functions contribute to their containing function too. Follows [Sonar Rust at `9b347aa`](https://github.com/SonarSource/sonar-rust/blob/9b347aa4298afac0460084be13db6671116152ed/analyzer/src/visitors/cyclomatic_complexity.rs). Macro token trees are opaque; macro expansion can hide branches. |
| Kotlin | Authored functions, accessors, constructors/init owners and callbacks. Named-function baseline/count stays unchanged: if/loops, each when entry (including else), logical AND/OR; nested named functions contribute to their container. Follows [Sonar Kotlin at `b10cb30`](https://github.com/SonarSource/sonar-kotlin/blob/b10cb30596e0874f5a7ad10180fafaf6c09e8ef3/sonar-kotlin-metrics/src/main/java/org/sonarsource/kotlin/metrics/CyclomaticComplexityVisitor.kt). PSI parsing does not require type resolution. |
| Python | Function baseline, if (excluding elif), for/while, conditional expressions, each logical operator and comprehension filter. Nested functions are scored separately. Follows [Sonar Python at `0161137`](https://github.com/SonarSource/sonar-python/blob/0161137cf31b6ead71293f5f35d5ced18f55bc0b/python-frontend/src/main/java/org/sonar/python/metrics/ComplexityVisitor.java); this intentionally differs from common generic Python counters. |
| C | Approximation: one plus if/for/while/do, case/default labels, ternaries and logical operators. Source-tree parsing covers conditional-compilation branches but does not expand macros or use Sonar's proprietary C analyzer. Compiler/preprocessor semantics can differ; a passing count is not native CFamily certification. |
| Shell | Maintainer-approved standard: one plus if/elif, loops, case alternatives and short-circuit operators. No native Sonar Shell metric is claimed. |

`test_metrics.py` permanently checks 9/10 boundaries for every language,
selected rule differences, parser rejection, embedded Python/package coverage,
and source exclusion. Rust macro opacity has an explicit fixture so it cannot
be mistaken for expansion-aware coverage. Architecture and responsibility
reviews remain necessary even when every function passes.

T843: before C syntax analysis, literal `__has_include` operands inside `#if`
and `#elif` directives become same-length numeric placeholders. No guard is
evaluated and both branches remain in the inventory. Source byte offsets and
line numbers remain unchanged; comments, ordinary string literals and executable
function tokens are untouched. Quoted/angle headers, continuations and nested
conditions are covered by permanent bounded tests. Malformed conditions/bodies
still fail closed. This adaptation addresses the pinned grammar's inability to
parse header-name tokens as preprocessor call arguments; it is not a C compiler.

T844: object-like annotation macros containing only compiler attributes (or
nothing) are recognized from all definitions in the partial syntax tree. The
adapter blanks only matching parsed declaration-type tokens at identical byte
offsets. Executable compound statements remain untouched. Conflicting definitions,
function-like macros and explicit undef prevent adaptation; unknown tokens and
malformed bodies still fail. This does not evaluate conditional compilation or
establish C compiler semantics. Archived owned source remains in the audit.

T965 extends Kotlin ownership beyond named functions. Coverage and complexity
share `scripts/kotlin/AuthoredKotlin.kt` for authored source boundaries, while
keeping native coverage attribution and complexity counting independent. Both
checker jars are keyed by their checker source plus the shared ownership source.

Each additional executable method starts at one and uses the same branch/logical
increments. Construction/static initialization does not descend into nested class,
named method or accessor declarations: those have separate owners, and constructing
a callback object does not execute its member methods. A primary constructor owns default parameter expressions, base-call
expressions, property/delegate initialization and init blocks, excluding unrelated
member declarations. Secondary constructors own their arguments/body and shared
instance initialization when they delegate directly to the base constructor; a
`this` delegation does not recount the shared initialization. Named object static
initialization and base-call construction have separate owners. Enum entry
arguments belong to the enum static owner; entry bodies remain instance owners.
Empty init blocks, literal-null default fields and generated empty construction
remain absent, as in the native coverage inventory.

Callbacks and anonymous functions receive their own source rows. PSI without type
resolution cannot decide which lambdas a compiler will inline, so this source
audit conservatively scores every authored lambda. Their branches retain their
existing enclosing named-function contribution, consistent with the pinned
recursive rule; this does not claim an independent native method for an inline
body. Authored accessors/constructors/callbacks use that same recursive count over
their owned bodies. Missing parse data fails closed. This is an independently
expressed source audit extending the pinned named-function metric's scope, not a
claim that Sonar's original visitor already enumerated these extra owners.
Permanent fixtures cover ownership, missing method kinds and bounded 9/10
boundaries. Keep newly exposed excessive methods in TODO until refactored.
