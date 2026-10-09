# Android review implementation

## T829: decoder replay generator prerequisite

The current-source offline decoder benchmark failed to compile because its
source list omitted `DecoderDiagnostics.kt`, while DecoderSession already used
its types. `test_decoder_diagnostics_dependency.py` failed before the source-list
fix and passes afterward. Generated `compileDebugKotlin` passes offline. This
repairs the benchmark only; production behavior is unchanged.
