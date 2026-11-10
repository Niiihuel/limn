# Material Icon Theme artwork

This directory contains an unmodified subset of **Material Icon Theme 5.38.1**,
copyright (c) 2025 Material Extensions, used under the [MIT license](LICENSE).
Product names and logos identify file formats and development tools; their
inclusion does not imply affiliation or endorsement.

- Source: https://github.com/material-extensions/vscode-material-icon-theme
- Release: https://github.com/material-extensions/vscode-material-icon-theme/releases/tag/v5.38.1
- Commit: `448ab3977ef83b817c2c722ce7cd5034d195b39f`
- Package: https://www.npmjs.com/package/material-icon-theme/v/5.38.1
- Original license: https://github.com/material-extensions/vscode-material-icon-theme/blob/448ab3977ef83b817c2c722ce7cd5034d195b39f/LICENSE

`SOURCE.json` pins the package archive, its SHA-512 integrity value, and each
vendored SVG's SHA-256. `scripts/vendor-file-icons.py` verifies that integrity,
copies 165 SVGs and the upstream license, and generates filtered filename,
extension, folder and language associations from the published manifest. It
does not install Node packages or execute their code. Open-folder SVGs are the
upstream package's generated assets. Unsupported artwork falls back to the
package's generic file/folder glyphs.

OpenIDE resolves names case-insensitively, preferring an exact filename and then
the longest matching compound extension. It accepts either slash style and
adds common code-fence aliases (`rs`, `c++`, `tsx`, `bash`, etc.). Runtime SVGs
are embedded with `include_str!`; resolution performs no filesystem access.
The native renderer caches the rasterized glyph per physical size and preserves
the artwork's colors and the host's disabled/transition opacity.
