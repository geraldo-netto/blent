# T1162 — landscape request and orientation evidence

Automatic orientation remains landscape-only, with saved pins and platform override limits. Documentation was corrected; hosted rotation tests do not qualify foldables, OEMs or multi-window behavior.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

sensorLandscape is an ongoing landscape policy, also the manifest default.
ActivityWindowPolicy applies the saved pin or automatic landscape request at
creation, on start and on setting change; automatic observation explicitly pins
one of the two landscape directions. It never requests portrait. stop disables
the owned sensor listener; manual pins stop observation immediately. Runtime
platform decisions can still ignore or letterbox the orientation request.

Correct compatibility.md's broad automatic-rotation wording and its implication
that a historical Tab S9 FE report qualifies current options. Preserve the
implemented landscape policy: lint DiscouragedApi is a form-factor advisory,
not evidence authorizing a new portrait policy. No suppression or code change.

The retained OrientationTest passed in T1161's ordered production-Activity
replay, three original tests on each SDK27/34, zero failures. It covers all four
display rotations for naturally portrait/landscape panels, unknown/dead-band
angles and manual pins. The immutable T1161 JUnit archive contains the nested
settled results and original assertions. Source hashes here identify that oracle.
Hosted tests do not qualify OEM, foldable or multi-window behavior. Physical
prerequisites remain T855/T857/T846; no device actions occurred.

</details>
