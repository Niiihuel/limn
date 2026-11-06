# Liquid glass

limn's glass samples a scene owned by the application. It keeps a sharp
copy for the refracted rim and a blurred copy for the middle. Foreground
text, icons and interaction remain ordinary elements.

## Aurora reference

The reference is [Aurora](https://github.com/Niiihuel/aurora), inspected at
`c13d8b3baf8d83e90be153bff1be74b62fce3578` on 2026-11-05.
The relevant source files are:

| File | Responsibility |
| --- | --- |
| `shell/components/Glass.qml` | Material, geometry, source mapping, local capture, press response and fallback. |
| `shell/components/GlassBackdrop.qml` | Wallpaper and overlapping-window reconstruction, sharp scene and shared blur. |
| `shell/components/GlassMorph.qml` | Drop-to-panel motion, material presence and content reveal. |
| `shell/shaders/glass.frag` | Outline, refraction, sharp rim, tint and lighting. |
| `shell/shaders/blur.frag` | One normalized Gaussian pass. |
| `login/Glass.qml` | The same optics over sharp and blurred wallpaper. |
| `scripts/shaders.sh` | Atomic compilation into Qt shader packages for GLSL, HLSL and Metal. |
| `plugins/rims/src/Shaders.hpp` | Related compositor-side optics for window rims. |

### Obtaining the background

Aurora does not read pixels behind its Wayland surface. `GlassBackdrop`
rebuilds the monitor scene from the wallpaper and screencopies of windows
on the active workspace. It filters captures to windows overlapping the
glass region, orders tiled windows before floating windows, and orders
those by focus history. An application can exclude its own process to
avoid feeding its glass back into the source.

Window captures refresh at 30 Hz by default. They stop while the glass
window is hidden or the region overlaps no window. This avoids a repaint
feedback loop that the source comments describe for live screencopy.
Wallpaper changes follow the wallpaper daemon's circular reveal.

The reconstructed scene produces two textures. The sharp texture has one
texel per logical point. The blur starts at one quarter of each logical
monitor dimension. Horizontal and vertical Gaussian passes use
`sigma = material.blur / 4` in that reduced texture. The loop reaches
`ceil(3 * sigma)` texels on each side, capped at 20, and normalizes the
weights. `blur` is a standard deviation in points, not a diameter.

For a lens over a control's own labels, `Glass.refracts` instead captures
that item's footprint with a margin for refraction. This source uses the
device pixel ratio and accounts for the lifted scale. The source must
exclude the glass itself. `login/Glass.qml` uses wallpaper images for both
sources because the greeter has no live window scene.

### Geometry and optics

The body is a rounded rectangle evaluated as a distance field. Corners use
superellipses: exponent 2 gives circular corners, and exponent near 4 gives
continuous corners. Capsules force exponent 2. An optional tooltip tail
joins a triangle to the body through a smooth union. Coverage is
antialiased using the device pixel ratio.

The outline gradient gives the outward normal. Across the rim, a convex
bezel changes the surface normal. Refraction uses an index of 1.5. Its
profile exponent varies from 2 for broad bezels to 4 for narrow bezels.
The displacement fades in at the outer edge, falls away toward the middle,
and is limited to 0.88 times the bezel width. A separate bezel width keeps
a capsule's middle flat. Magnification acts on the middle independently.

Five samples along the bent direction soften the compressed background.
Toward the rim, three samples of the sharp scene are blended in, with
weight `0.9 * (1 - depth)^1.6`. This is central to the appearance: bending
only a heavily blurred source conceals the distortion. Optional dispersion
offsets red and blue in opposite directions for both sources.

Saturation uses gamma RGB and luminance weights `(0.213, 0.715, 0.072)`.
The shader clamps to premultiplied alpha, applies optional dimming, then
mixes in the tint. A faint dark line inside the edge seats the surface.
A directional hairline faces the top-left, with weaker light on the far
side. A wider exponential specular band catches light across the curved
rim. The output uses premultiplied alpha. A uniform translucent border
alone cannot reproduce this lighting.

The regular shell preset uses blur sigma 8 points, refraction 32 points,
dispersion 0.5 points and saturation 2.2. The login wrapper uses refraction
14 points. The shader's bezel bound still limits their effective bend.
Colors come from Aurora's theme; these measurements describe the reference
and are not instructions to put literal colors in limn components.

### Material and motion

`Glass.qml` grows on press using a spring, blends toward a lifted material,
and moves a soft highlight with the pointer. Reduced motion removes the
growth. Reduced transparency, or an unavailable source, uses an opaque
fill. Its analytic shadow is separate from the optical shader.

`GlassMorph` coordinates geometry and material. Opening first creates a
small round drop near the trigger, then springs to the destination panel.
The corner radius moves from half the shorter side to the panel radius.
The veil gains presence as the shape forms. A short bloom accompanies the
opening, while the content fades and focuses in from blur after the shape
begins forming. Closing shrinks to the drop, then merges into the trigger;
content blurs out early and the returning drop retains its veil. Changing
the target while open reshapes the same surface. Motion is separate from
the shader and must use limn motion tokens rather than copied durations.

## What the removed renderer established

Commit `07126ef` removed the previous implementation on 2026-10-19. Its
parent contains `crates/limn-egui/src/glass.rs`, `glass.wgsl`,
`crates/limn/src/glass.rs` and the previous version of this guide. The
removal does not document the author's reason; no reason is inferred here.

That implementation already accepted RGBA and GPU backdrops, replaced a
named `Surface` at its paint depth, reused allocations and shared Gaussian
blur results. Large radii used repeated 2x reductions before separable
filtering. These are useful resource-management techniques.

It required the application to supply the scene. Applying a material did
not capture the actual page. Its optical path sampled the prefiltered
scene rather than mixing a separate sharp source into the bent rim, and
its corner field did not expose Aurora's superellipse profile. Its guide
explicitly left GPU timing unmeasured. Restoring those files alone would
therefore leave the central integration and fidelity gaps open.

## Rendering boundary

`egui-wgpu` 0.37 prepares callbacks before drawing the window's egui pass.
Its window surface has `RENDER_ATTACHMENT` usage. A callback cannot sample
already-painted window content. Adding a glass command does not change
that constraint.

The application must render the page into a sampleable offscreen texture,
or supply an existing scene texture from the same wgpu device. The glass
then samples that texture and its blur before foreground controls are
painted. Video must be included in this source through the existing
`SurfaceFrame` / `present_surface_paint` / `present_surface_texture` bridge.
A texture containing only artwork cannot refract a video drawn elsewhere.

External paint callbacks need an explicit capture contract: an arbitrary
either-backend callback cannot be assumed to render correctly into another
target. Exclude the glass itself from its backdrop to avoid feedback.

`GlassRenderer::paint_page` does this for a whole window: it renders what a
frame paints beneath its first glass into the backdrop, then paints the frame.
The capture is drawn by the window's own egui renderer from the window's own
textures, on a layer whose shapes are taken before the window draws them.
Pictures and CPU surfaces such as a video's frames therefore appear as they do
on screen, with no second decode or upload: a second egui context would decode
every picture again, and limn's image and surface caches hold one context's
textures at a time. Paint callbacks are left out of the capture. The window's
renderer must draw without multisampling or depth, as eframe's does by
default. A frame without glass is painted without any capture.

## Integration in Surco

Surco remains unchanged by this branch. Its intended application is the
floating tab bar, mini player, pop-up menus and phone bottom sheets.
`docs/bitchord-look.md` records the current opaque approximation.

In `crates/app/src/view/mobile.rs`, `floating_bar()` builds both navigation
and mini-player capsules. Replace its `background(FROST)` with the glass
material, retaining sizing, child clipping, stable IDs and hit regions.
Use the dark theme's material tint. Keep foreground glyphs and the selected
tab platter outside the backdrop source. Reassess the existing uniform
hairline because the optical shader draws its own directional rim.

The `FROST` shortcut comment can be removed when the real capture path is
wired. Audit its other uses, including the floating header controls, before
removing the constant. A material on a parent around a `BottomSheet` would
also cover its base page: apply the material to the sheet's actual surface.
The same applies to pop-up menu surfaces. Keep scrims separate from glass.

Wire the host's page capture before painting the floating surfaces. Include
the live video `SurfaceFrame` at its original depth, clip, opacity and UV
mapping. Reuse GPU textures; do not read video back to CPU or upload a second
RGBA image for each glass. Ensure the page's bottom fade does not obscure
all detail behind the bars, or correct glass will still look flat.

After review and publication of the limn commit, update `LIMN_REF` to that
same full commit ID in all five Surco workflows:

| Workflow | Pin |
| --- | --- |
| `.github/workflows/ci.yml` | `env.LIMN_REF` |
| `.github/workflows/android.yml` | `env.LIMN_REF` |
| `.github/workflows/ios.yml` | `env.LIMN_REF` |
| `.github/workflows/release.yml` | `env.LIMN_REF` |
| `.github/workflows/testflight.yml` | `env.LIMN_REF` |

The inspected pin is `1cab09c1af96533116b91debf547c4f5d72e3122`.
The branch name is not a reproducible release pin. No workflow or Surco
source is edited as part of this work.

## Platform and performance limits

The rendering path must fit downlevel phone limits: ordinary sampled 2D
textures and render passes, without compute, storage textures or mandatory
timestamp queries. Surco requests `Limits::downlevel_defaults()` on Android
and iOS and caps texture dimensions to the supported limit up to 8192.
Respect the actual device limit when allocating the scene and blur pyramid.

A 60 Hz frame lasts 16.67 ms, including video decoding, page drawing and
composition. A desktop timing cannot establish that budget on a midrange
phone. Measure the extra glass cost with the same scene, dimensions, device
pixel ratio and surface count with glass enabled and disabled. Report CPU
submission and GPU completion separately when available. A synchronized
software-GL wall-clock benchmark is useful for QA but is not a phone GPU
time. Include offscreen capture and blur in the cost, and state whether the
background changes each frame.

Device checks must cover Vulkan/GLES on Android, Metal on iOS and desktop,
and `WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1` for QA. A shader validation test
alone is not evidence of those devices running. Measurements and visual
artifacts for this branch are recorded below after implementation checks.


## Motion reference and component contract

The additional reference is the user's local BitChord checkout at
`~/proyectos/personal/referencias/BitChord`. Its GPL source is read for
behavior and measured parameters. Component implementations in this branch
are written for limn's retained IDs, elements and existing scalar springs;
BitChord's source is not copied.

| Reference | Observed parameters and behavior |
| --- | --- |
| `IosSwitch.kt` | Position stiffness 700, damping ratio 0.78. Press stretches width to 1.18 with stiffness 900 and damping ratio 0.7. Track 64 × 28, thumb 40 × 24 with 2-point inset. Its current drawing is flat white, so a refracting glass thumb extends beyond this reference. |
| `floatingtabbar/GlassSelectionPill.kt` | Travel stiffness 280, damping coefficient 31.4. Lift stiffness 250 with damping ratios about 0.60 across and 0.70 vertically. Material responds faster with stiffness 1000 and critical damping. The lift releases after 92% of travel. Acceleration deformation is limited to 12%. Retargeting retains velocity. The lens samples the page, bar and recorded tab row. |
| `IosMenu.kt` | Critically damped opening spring with stiffness 600; ordinary close uses a 170 ms eased transition. The surface scales about the right-hand corner, above or below according to placement. |
| `HeldContextMenu.kt` | The held row lifts while an attached menu grows from its nearest corner. Opening stiffness 420, damping ratio 0.72; closing is 230 ms. |
| `floatingtabbar/FloatingTabBar.kt` | Scroll direction accumulates until a threshold; reversal resets the accumulation. A shared player moves between expanded and inline layouts; selected tab and separate search stay reachable. |
| Aurora `shell/core/Motion.qml` | Opening is a 60 ms drop hold followed by a 260 ms perceptual spring with bounce 0.30. Shape changes during 200–333 ms, content focuses during 233–400 ms. Closing starts merging at 170 ms. Press uses 300 ms perceptual duration and bounce 0.35; landing uses 420 ms and bounce 0.55. |

These sources describe distinct animations. A 170 ms phone-menu target
must not be described as an exact reproduction of Aurora's longer phased
menu. Perceptual spring duration, damping coefficient and damping ratio
are different quantities. The implementation uses `ScalarSpring` and
`SpringMotion`; it does not run a second numerical integrator copied from
the reference.

The video is the user's local
`surco/.claude/reference/bitchord/video.mp4`, 588 × 1274 at 30 fps. Frames
are read from that path without writing to Surco. Relevant intervals are:

| Interval | Comparison |
| --- | --- |
| 22.8–23.3 s | Context menu growing from the more button. |
| 34.9–35.6 s and 44.8–45.5 s | Selection moving to Library and Explore. |
| 37.0–38.0 s and 46.5–47.5 s | Inline bar during scrolling. |
| 13.4–14.1 s | Bottom sheet entering. |

A full-video contact sheet contains no interactive switch sequence.
Switch fidelity therefore cannot be established against this video; its
spring and press behavior are compared with the source reference instead.
At 30 fps, each reference frame spans 33.33 ms. A strip can reveal position,
shape and reveal order; it does not prove subframe curve identity.

`FloatingGlassBar` keeps one player subtree and its action IDs while the
player capsule narrows and moves between rows. `FloatingBarMotion` is
host-owned state driven by scroll intent, retaining spring velocity when
that intent reverses. `GlassMenu` uses the existing dropdown behavior for
focus, keyboard navigation and dismissal. Its origin rectangle comes from
the trigger's laid-out bounds, or a held row's bounds. Reduced motion must
snap host-owned motion as well as the `UiTree` transitions.

## Reproducing the Aurora capture

`scripts/capture-aurora-glass.sh` compiles the original reference shaders
and runs a small QtQuick harness over a deterministic scene. Supply a
separate Aurora checkout and an output directory. It runs no Quickshell
services and changes no desktop configuration. Qt 6 shader tools and a
QtQuick runtime are required; `QSB` and `QML` select their executables.

```sh
QSB=/path/to/qsb QML=qml6 bash scripts/capture-aurora-glass.sh /tmp/aurora /tmp/aurora-glass-capture
```

The capture uses native 720 × 480 output, device pixel ratio 1, the same
raster backdrop as limn's comparison, and three surfaces: a capsule at
`[40,48,360,112]`, a continuous-corner panel at `[245,205,390,180]` with
radius 32, and a smaller panel at `[500,54,130,100]` with radius 24. The
comparison tint quantizes Aurora's dark regular preset to RGBA8
`(38,38,40,122)`. Blur sigma is 8 points, refraction 32 points, dispersion
0.5 points and saturation 2.2. This holds input and geometry constant rather
than comparing unrelated application screenshots.

The Qt RHI path is required because the pure QtQuick software scene graph
skips shader effects. The QA capture ran with Mesa software OpenGL through
RHI. `QT_FONT_DPI=96` avoids the monitor's fractional device scale changing
the capture dimensions. No system packages were installed: the missing
shader compiler was extracted into a temporary directory.
