# Bootstrap Icons

The SVG files in this directory are unmodified icons from
[Bootstrap Icons](https://icons.getbootstrap.com/) 1.13.2, taken from the
`bootstrap-icons` npm package. They are the library's standard iconography.

MIT license, copyright 2019-2024 The Bootstrap Authors. See [LICENSE](LICENSE).

Each icon is a 16 px square drawn with `currentColor`. The backend rasterizes
it once per physical size as a white mask and tints it when painting, so one
texture serves every theme and state.

`src/lib.rs` maps each `limn::Icon` to a file here. To add an icon, copy its
SVG from the package and add the mapping.
