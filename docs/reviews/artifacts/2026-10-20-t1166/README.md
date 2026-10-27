# T1166 — themed launcher layer

A monochrome adaptive-icon layer reuses the existing geometry. Packaged resource checks passed; physical themed-launcher appearance remained unvalidated.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Add a monochrome layer to the existing adaptive icon and a vector reusing its
108dp canvas and all original path contours. The tablet screen becomes an even-odd
alpha cutout; existing window-bar alpha and stylus geometry remain. No raster,
new branding or external asset is introduced. The original background/foreground
references and the original T1164 test method remain byte-identical.

The permanent API34 packaged-monochrome regression failed before the layer,
then passed unchanged. SDK27/34 still inflate the adaptive icon with both original
layers. Cached API27 framework bytecode skips unknown child names, so one cohesive
adaptive XML serves both versions; the final APK has one compiled adaptive entry
with all three layers. No redundant versioned layer definition remains. Geometry
comparison, compiled aapt2 tree, sealed red/green JUnit and exact logs follow.

Final offline build runs5 packaged tests with zero failures/skips, assembles the
APK and reports lint0errors6warnings with MonochromeLauncherIcon removed. New test
complexity is1 in the cached PSI1.9.20 source audit; native Sonar remains T1019.
Static resources add no app-owned handle, callback or mutable identity: the resource
loader owns bounded drawable inflation and the existing icon lifetime. Physical
themed-launcher appearance is unvalidated; hosted resource checks do not establish
OEM rendering. No device, network transfer, installation or push occurred.

</details>
