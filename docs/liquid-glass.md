# Native glass navigation

Design notes for a Ricelin-style navigation bar in limn, researched on
2026-10-13. The goal is an interactive native Rust component with optical
glass and limn's animation system. Desktop-shell integration is a separate
stage from rendering glass inside an application.

## Try the component

```sh
nix-shell --run 'cargo run -p limn-gallery -- --page glass'
```

Drag the handle below the bar to move it over the preview, or use the
horizontal and vertical sliders. Escape cancels a drag; the reset button
restores the initial position. The bar's buttons remain clickable after moving.
Six backgrounds are available: amber, ocean, lines, checkerboard, rings and
content. The content scene contains actual limn text, icons and cards, making
it easier to inspect refraction across contrasting edges.

Background animation starts enabled; pause it or adjust its speed independently
of the bar. Reduced motion also pauses the background. Optical controls,
compact width and opaque mode remain available below the preview. Workspace
clicks update the active capsule; status actions open simulated panels.
The gallery does not replace the running Ricelin shell or query live services.

```rust
use limn::{Color, Component, GlassMaterial, Icon, NavigationAction,
           NavigationBar, NavigationWorkspace};

let bar = NavigationBar::new("desktop", palette)
    .workspaces(vec![
        NavigationWorkspace::new("workspace:1", "Workspace 1").selected(true),
        NavigationWorkspace::new("workspace:2", "Workspace 2"),
    ])
    .clock("11:06:46", "TUE 13 OCT")
    .clock_action("calendar")
    .actions(vec![NavigationAction::new("settings", Icon::Settings, "Settings")])
    .width(900.0)
    .glass(GlassMaterial::new(Color::rgba(24, 21, 22, 105)))
    .render();
```

Handle `UiAction::Clicked` with the supplied action ID, then rebuild with the
new selected workspace or active action. `NavigationBar` derives its focus
IDs from its own ID, so several bars can dispatch the same host actions.
Workspace arrows skip disabled entries; Tab and Enter reach status controls.
Narrow bars hide secondary text and scroll to reveal keyboard focus.
The selected capsule uses `move_ms`, control fills use `hover_ms`, and
`UiTree::set_reduced_motion(true)` stops those transitions. Omit `.glass`
for reduced transparency. This first component is the expanded navigation
row; Ricelin's compact-clock hover morph and traveling bead remain future
shell work.

## Supply the backdrop

`GlassMaterial` is backend-neutral and can be applied to another element
with `Element::glass`. Its builders bound blur, refraction and dispersion
in logical pixels. Text, icons and hit regions remain ordinary limn elements.

Create `limn_egui::GlassRenderer` from the creation context's wgpu render state,
then upload an image with `set_backdrop_rgba([width, height], &rgba)`. Reuse
the renderer and upload again only when pixels change. `paint_at` takes the
frame, its egui origin and the window rectangle occupied by that image.
The host must show the same image in that rectangle: `paint_backdrop` can do
this, or `paint_at_surface` can replace a named limn `Surface` with the shared
texture at that surface's original paint depth, clip and opacity.

For animated native scenes, `set_backdrop_texture` adopts a texture created
on the same wgpu device, without CPU readback or per-frame image uploads.
It accepts single-mip, single-sample, single-layer 2D `Rgba8Unorm` textures
with `TEXTURE_BINDING` usage. Pixels use premultiplied gamma RGBA8; an opaque
scene is simplest. Submit the scene's render pass before the egui draw.
Invalid texture metadata preserves the previous backdrop.

The gallery's [`glass_demo.rs`](../crates/limn-gallery/src/glass_demo.rs)
renders the background into one shared GPU texture, capped at 2048 pixels
per dimension while preserving its logical aspect ratio. The content atlas
is rendered once through limn and egui; WGSL animates the scene. Each moving
frame updates a 16-byte uniform, not a CPU image. Textures are reused until
the preview size changes. A paused unchanged scene is not redrawn, and a
preview outside the viewport does not request continuous repainting.

The backdrop is softened in horizontal and vertical Gaussian passes before
refraction. Large blur radii use a sequence of 2× reductions first, bounding
each Gaussian pass to 25 texture samples per pixel. Surfaces with the same
radius and backdrop scale share the result within a paint call; active
allocations are reused, and unused radii are discarded so moving a slider
does not accumulate textures. An animated input is filtered again each paint.
Zero blur samples the original backdrop directly.

The material's numbers come from the stylesheets of Apple's web apps
(Music, Podcasts, TV and App Store share them): `saturate(220%) blur(16px)`
under a veil of `#f5f5f7` at 55% (light) or `#262628` at 60% (dark), a
half-pixel inner stroke of black at 5% or white at 20%, and a
`0 10px 40px` shadow of black at 10% or 20%. `GlassMaterial::light()` and
`dark()` carry the veils; blur is a radius of twice the deviation, so 32.

Those pages do not refract. The lens follows the native material in the
video below instead: the rim is a convex squircle bezel as wide as the corner
radius, and a ray of index 1.5 through it is displaced by up to that width,
so the rim is thin and shows the middle of the shape mirrored. The bend
starts from nothing at the edge: what is outside runs on into the glass,
squeezed, before it mirrors, which is how text and color enter a shape in
the 11:46–11:58 toolbar sequence. The native
material also blurs far less than the web one, hence the default radius of 8.
Dispersion is ours.

The edge follows iOS 27 as shown in the
[Platforms State of the Union, WWDC26](https://developer.apple.com/videos/play/wwdc2026/102/)
at 22:46–22:58, where Apple wipes between the old and new tab bar: "a
darkened edge along with brighter specular highlights". We draw a bright
line one pixel wide, strongest where the rim faces the light and on the
opposite side, with a faint darkened line just inside it. Five lookups along
the bend keep the squeezed rim soft. The same talk adds a system slider from
ultra clear to fully tinted; the gallery's "Tinte" slider is the veil's alpha.

The optical shader uses an antialiased rounded distance field, refracted
texture coordinates, optional chromatic separation, saturation, tint, a bright
edge line and a darkened one inside it. It samples the prefiltered texture once,
or three times with dispersion. This replaces the initial sparse nine-tap
approximation, which visibly repeated background letters in the content
scene. GPU timing remains to be measured on the target system.
The current pipeline uses eframe's default render target without MSAA or a
depth attachment. Ordinary `limn_egui::paint_at`, or a renderer without an
uploaded backdrop, paints an opaque tint fallback. Invalid uploads return
`false` and preserve the previous GPU image.

## Existing Ricelin behavior

The reference lives in `~/dotfiles/configs/quickshell/pill/`. It is already
a native QtQuick/Quickshell shell on NixOS and Hyprland. Its central object
is one morphing pill, rather than a permanent bar with separate windows
for every panel.

| Source under `~/dotfiles/` | Behavior to preserve |
| --- | --- |
| `configs/quickshell/pill/Pill.qml` | Compact clock at rest; hover expands the status row; clicking the pill pins it. The same body changes width, height and radius for each open surface. |
| `configs/quickshell/pill/Workspaces.qml` | Workspace dots belong to a monitor; the active dot becomes a short capsule; clicking focuses that workspace. |
| `configs/quickshell/pill/PillSurface.qml` | Incoming content fades in as the body approaches its target size; a settled surface stays visible while its content changes height. |
| `configs/quickshell/pill/shell.qml` | Two layer-shell surfaces per monitor: a small exclusive reservation and a transparent overlay. Input regions, outside clicks, Escape and fullscreen hiding are handled here. |
| `configs/quickshell/pill/Singletons/Motion.qml` | Shared timing: 140 ms feedback, 300 ms standard, 420 ms body morph, 820 ms bead transformation; the body uses cubic-bezier `(0.16, 1, 0.3, 1)`. |
| `configs/quickshell/pill/Ame.qml` | A luminous bead travels between controls with anticipation, a curved flight, droplets and a settling animation. |
| `configs/quickshell/pill/Tooltip.qml` | Hints appear after 470 ms, usually below the icon; they never take input. |
| `configs/quickshell/pill/Singletons/Theme.qml` | Inter typography, semantic colors, a static dark palette or a palette derived from the wallpaper. |

The expanded row orders workspaces, clock/date, weather, optional trays,
connectivity, battery, notifications, audio, system monitor, Docker,
recording, settings and power. A media button extends from the right edge.
Clock and weather open the calendar; connection icons choose their network
subview; the other status icons open their matching surfaces. Recording
also supports right-click or double-click to stop. Restart and shutdown
use a sustained press, including on the keyboard.

The reference reserves the compact height even while expanded, so opening
a panel does not move tiled windows. A 300 ms hover-exit grace period avoids
collapse while the body moves under the pointer. The expanded layout is
scaled per monitor and can hide optional indicators.

The screenshot's brown material is a wallpaper-derived gradient, border,
shadow and highlight. Its current body opacity is 1.0. Hyprland is configured
to blur the `pill` layer, but an opaque body hides that backdrop. There is
no optical refraction in the QML body today; its liquid quality comes from
shape and motion.

The dotfiles are the running configuration: `modules/home/ricelin.nix`
links Hyprland and Quickshell files outside the Nix store, and Quickshell
reloads edits. Develop the first component in this repository before
changing the desktop configuration.

## Rendering direction

limn's core already separates layout, input, animation and drawing commands
from the host. `limn-egui` uses eframe's wgpu renderer. Extending that path
keeps the component native without introducing a web view or a second UI
toolkit. See the [backend contract](backend.md) and [animation model](animation.md).
wgpu targets native Vulkan, Metal and D3D12 as well as the web; using it
does not imply a browser. See [wgpu's supported backends](https://github.com/gfx-rs/wgpu).

| Approach | Fit for this project |
| --- | --- |
| WebGL in a browser | Useful optical techniques, but would add a browser host that the project does not need. Shader mathematics can be adapted without adopting its runtime. |
| Native wgpu and WGSL | Preferred extension of the existing GPU backend; supports explicit textures and render passes while keeping layout and interaction in Rust. |
| Direct Vulkan/OpenGL | Can implement the effect, but introduces graphics-backend work already handled by wgpu. |
| Compositor blur with translucent paint | A practical desktop fallback. Blur and tint alone do not refract background geometry. |

Rust does not remove the need for GPU shaders or make every rendering path
automatically faster. The useful advantage here is control over the native
host, resources and frame scheduling while reusing the current backend.

Apple describes Liquid Glass as a navigation/control material with lensing,
reflections, adaptive appearance and coordinated motion. These are the
design goals, rather than simply increasing transparency.
See [Meet Liquid Glass](https://developer.apple.com/videos/play/wwdc2025/219/).

The proposed material pipeline is:

1. Obtain a backdrop texture from content that is actually behind the glass.
2. Build a blur texture, preferably at reduced resolution, and retain an
   unblurred source for controlled edge detail.
3. Evaluate a rounded-rectangle distance field and a smooth edge profile.
   Use the profile to displace backdrop sampling, producing lens-like
   refraction near the contour.
4. Combine blur, a restrained tint and edge highlights. Keep text and icons
   sharp by painting them after the material.
5. Animate the shape and interaction state through limn; keep the optical
   material attached to the same moving geometry.

In egui-wgpu, callback preparation runs before the main egui render pass.
It cannot assume that earlier UI paint commands have already produced a
sampleable background. Pass an explicit texture or prerender the backdrop
offscreen; see the [callback contract](https://docs.rs/egui-wgpu/0.36.2/egui_wgpu/trait.CallbackTrait.html).

This is an approximation inspired by Apple's material, not a claim to
reproduce Apple's private implementation. First validate refraction over
a detailed moving background; a highlight over a flat fill is insufficient
evidence. Strong dispersion, oversized glints or excessive distortion
should remain optional, since they compete with small navigation labels.

## The desktop backdrop boundary

A renderer can sample pixels in its own textures. Transparency in a Wayland
window lets the compositor show other windows through it, but does not
hand those windows' pixels to the client shader. The compositor owns the
final output; see [Wayland architecture](https://wayland.freedesktop.org/docs/book/Architecture.html). A normal application
backdrop and the desktop behind a layer-shell panel are different inputs.

Layer-shell provides placement, stacking, reserved space and keyboard
interaction, per the [layer-shell protocol](https://raw.githubusercontent.com/swaywm/wlr-protocols/master/unstable/wlr-layer-shell-unstable-v1.xml).
It does not provide an arbitrary desktop backdrop texture.
The existing Hyprland blur rule can blur what is behind a translucent panel;
it does not expose that background for the panel's own refraction shader.
See [Hyprland's layer rules](https://wiki.hypr.land/configuring/core/rules/layer-rules/).

For a desktop version, choose and document the source explicitly:

- An owned wallpaper texture supports correct wallpaper refraction, but
  excludes other applications.
- Compositor blur plus local highlights is a useful fallback when no
  sampleable desktop backdrop is available.
- A compositor integration could supply the scene before the panel is
  composed. A capture-based alternative needs separate evaluation for
  permissions, latency, scaling and self-capture feedback. Standard capture
  sources address [outputs or toplevels](https://raw.githubusercontent.com/wayland-mirror/wayland-protocols/main/staging/ext-image-capture-source/ext-image-capture-source-v1.xml).

Do not describe the standalone component as desktop refraction until that
integration has been built and verified with moving windows behind it.

## Migration in three stages

1. **Material and component.** Build the navigation row and glass material
   in a normal application with a known backdrop. Preserve useful IDs,
   hover/press/focus behavior, tooltips and interrupted transitions. Use
   mock service data; actions should update the example rather than change
   system settings. Verify clipping, scale factors, resize, reduced motion
   and readable foreground contrast. Keep an opaque fallback.
2. **Layer-shell host.** Add per-monitor surfaces, exclusive zones, input
   regions, keyboard focus, Escape, outside dismissal and fullscreen
   behavior. Evaluate the backdrop source independently of placement.
   Verify that transparent areas pass input through and opening a panel
   never resizes the desktop's tiled windows.
3. **Service adapters.** Connect state and semantic actions to Hyprland IPC,
   NetworkManager, Bluetooth, UPower, PipeWire, MPRIS, notifications and
   system tray. Add Docker and recording as optional adapters. Preserve
   unavailable-service states and deliberate confirmation for power actions.

Measure GPU time and repaint frequency on the target system before calling
the result lightweight. Reuse textures; avoid full-desktop processing for
a small panel; render at the display cadence during transitions and stop
unnecessary work when both the backdrop and controls are unchanged.

## Verification of this first implementation

On 2026-10-13, `cargo test -p limn -p limn-egui -p limn-gallery` passed
950 tests with 2 existing ignored tests; `cargo check --workspace` also passed.
Coverage includes workspace actions,
keyboard reveal in narrow bars, reduced motion, numeric validation, draw
transforms, upload bounds, backdrop coordinates, shader validation and its
uniform layout, drag/cancel followed by button activation, responsive preview
controls, animation pause/resume, atlas font initialization and blur scaling.
Native wgpu/Vulkan screenshots were inspected in dark and
light themes, with a status panel open, and with compact width, opaque mode
and square design tokens. The content scene was checked over text and icons
at both large and small blur settings after replacing the sparse blur.
Strict Clippy remains blocked by existing warnings
in unrelated components; it is not claimed as a clean check. GPU timing and
moving desktop backdrops have not been benchmarked.

## Video study

On 2026-10-13 we inspected selected sequences from Apple's
[Meet Liquid Glass, WWDC25](https://developer.apple.com/videos/play/wwdc2025/219/),
using its official
[SD video download](https://devstreaming-cdn.apple.com/videos/wwdc/2025/219/4/476dabc8-f3bb-4190-8929-4ba7131c939d/downloads/wwdc2025-219_sd.mp4?dl=1).
`ffprobe` reports 960 × 540 at 30000/1001 fps. Overview sheets sampled every
two seconds; detailed sheets contain 16 consecutive source frames each,
33.367 ms apart, read left-to-right then top-to-bottom. This examines selected
excerpts, not every frame of the complete talk. Extraction used HTTP seeking
with `ffmpeg -ss <seconds> -i <url> -vf 'scale=480:-1,tile=4x4' -frames:v 1`.
Reference images stay in `/tmp`; no Apple assets are included in the project.

| Consecutive excerpt | Direct visual observation |
| --- | --- |
| 04:28.502–04:29.002 | The dragged slider lens changes width and curvature. Its center retains a recognizable blue track; the curved rim displaces it. |
| 05:01.501–05:02.002 | A capsule expands into joined lobes before the new controls separate. Highlights and content clarity change during the transition. |
| 11:49.509–11:50.009 | A floating toolbar crosses text. The background text softens and bends while the toolbar's foreground icons remain sharp. |

The wider 11:40–12:12 sequence also shows stronger shadow where the toolbar
overlaps text and weaker shadow over the empty background. The talk describes
this adaptation explicitly; exact shader parameters cannot be inferred from
these compressed frames.

Engineering implications for limn: provide independent movement of the lens
and background, include real text/icons and high-contrast edges in the test
scene, and keep foreground glyphs outside the sampled backdrop. A clearer
center with stronger edge refraction, restrained deformation during dragging,
and smoothly changing light/shadow are useful next experiments. These are our
implementation recommendations, not a reconstruction of Apple's renderer.
