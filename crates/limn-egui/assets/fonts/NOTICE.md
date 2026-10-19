# Ubuntu Regular, Medium and Bold

Ubuntu-Regular.ttf, Ubuntu-Medium.ttf and Ubuntu-Bold.ttf are distributed
unmodified under the Ubuntu Font Licence 1.0. The full licence and copyright
notice are included in UFL.txt.

Source: https://github.com/google/fonts/tree/a50de97857f6626d6628c91e6a34c12ee8a16ede/ufl/ubuntu

SHA-256:

- Ubuntu-Regular.ttf `3128df86a31805618436d0ae5651ba4285d0c9de0a39057d025f64ee33bceb64`
- Ubuntu-Medium.ttf `3cd952b8b52581e48a8fa95b31c8829c2baa41b635042a69587d57f98092961e`
- Ubuntu-Bold.ttf `679b5c1e09cab3156bb8ef529735f9382bf31ca7ac737382ab959297f8d82ad4`

limn-egui uses the Regular face for proportional UI text instead of egui's
default Ubuntu Light face, and the Medium and Bold faces for
`FontFamily::SansMedium` and `FontFamily::SansBold`: real faces, never a
synthesised weight. Font hinting, antialiasing and display scaling continue
to use egui's defaults.

# Ubuntu Sans SemiBold

UbuntuSans-SemiBold.ttf is the unmodified static face from the Ubuntu Sans
fonts 1.006 release, distributed under the same Ubuntu Font Licence 1.0 as the
faces above (UFL.txt; the release's LICENCE.txt is identical byte for byte).
Copyright 2010, 2011 Canonical Ltd.

Pinned release:
https://github.com/canonical/Ubuntu-Sans-fonts/releases/tag/v1.006

Archive:
https://github.com/canonical/Ubuntu-Sans-fonts/releases/download/v1.006/UbuntuSans-fonts-1.006.zip

The archive member is `UbuntuSans-fonts-1.006/ttf/UbuntuSans-SemiBold.ttf`.
The font's name table reports `Version 1.006; ttfautohint (v1.8.4.7-5d5b)`.
It adds 487,284 bytes, with no subsetting or conversion. SHA-256:

- Release archive `20aa7af47beaa6d64327782f7bf60df375edeedbdee2fe407e0bf9721890fb83`
- UbuntuSans-SemiBold.ttf `b0930ffaa33588438a2f45fae3fdf779ec2c0ffd1b71ac4e9b80baf36734e13e`

The Ubuntu family has no semibold weight; Ubuntu Sans, its successor from the
same project, does. limn-egui uses it for `FontFamily::SansSemibold`, a real
face with the same fallbacks behind it as the other weights. Its letterforms
are close to Ubuntu's but not identical, so it is used where a semibold is
asked for, not in place of Medium or Bold.

# Noto Sans Math

NotoSansMath-Regular.ttf is distributed unmodified under the SIL Open Font
License 1.1 (SPDX: OFL-1.1). Its copyright notice and full license are in
NotoSansMath-OFL.txt. Copyright 2022 The Noto Project Authors.

Upstream project: https://github.com/notofonts/math

Distribution source, pinned to a Google Fonts commit:
https://github.com/google/fonts/tree/823468bd7825152bce2b8fd2cf740432ad2fce8d/ofl/notosansmath

The font's name table reports `Version 3.000; ttfautohint (v1.8.4.7-5d5b)`.
The bundled font is 1,015,396 bytes. SHA-256:

- NotoSansMath-Regular.ttf `3f495fe933c06786e4d5f6d86b8ee70b6753a68ee3b9d87528726de0f6e2c47d`
- NotoSansMath-OFL.txt `5b18c06210e575a638ec293b5c824a4e1084a7ec5a3d67098b15dae51d2df24b`

The license text is preserved; trailing whitespace is trimmed.

limn-egui appends this face after egui's existing fallbacks in proportional,
weighted and monospace families. It covers mathematical alphabets that the
UI faces lack; it does not replace their existing glyphs or add color emoji
or general CJK coverage. Hosts distributing the embedded font must retain
its copyright and license, independently of limn's code license.

# Noto Sans Symbols 2

NotoSansSymbols2-Regular.otf is the unmodified, unhinted OpenType face from
Noto Sans Symbols 2 v2.008. Copyright 2022 The Noto Project Authors
(https://github.com/notofonts/symbols). It is distributed under SIL Open Font
License 1.1 (SPDX: OFL-1.1); the complete copyright and license text is in
NotoSansSymbols2-OFL.txt.

Pinned release:
https://github.com/notofonts/symbols/releases/tag/NotoSansSymbols2-v2.008

Archive:
https://github.com/notofonts/symbols/releases/download/NotoSansSymbols2-v2.008/NotoSansSymbols2-v2.008.zip

The archive member is `NotoSansSymbols2/unhinted/otf/NotoSansSymbols2-Regular.otf`.
The font's name table reports `Version 2.008`. It adds 404,576 bytes; there is
no subsetting, conversion or runtime download. SHA-256:

- Release archive `346c930bbe8eb946701a05c54e9c11a2094dee1d93c387bf1771c0a3e335688f`
- NotoSansSymbols2-Regular.otf `9b0f186fea27f311018800fb3a01d44d59331dbcccf518d2dadcae894eb017c0`
- NotoSansSymbols2-OFL.txt `e87c2ed7ff174c637d55fa381939ebb96f43f0415ad94605a37589228f4cbf4f`

The license text is preserved; one trailing space is trimmed.

It follows the existing faces and Noto Sans Math in every family, supplying
missing monochrome decorative symbols. Existing text keeps its face. This
does not provide complete Unicode, shaping or color emoji support.

# Noto Emoji

NotoEmoji.ttf is the unmodified monochrome variable font distributed as
`NotoEmoji[wght].ttf` by Google Fonts. Copyright 2013 Google LLC. It is
distributed under the SIL Open Font License 1.1 (SPDX: OFL-1.1); the full
copyright and license text is in NotoEmoji-OFL.txt.

Upstream project: https://github.com/googlefonts/noto-emoji

Distribution source, pinned to a Google Fonts commit:
https://github.com/google/fonts/tree/8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5/ofl/notoemoji

The name table reports `Version 3.002`. The font adds 1,982,596 bytes, with
no subsetting, conversion or runtime download. SHA-256:

- NotoEmoji.ttf `de6c18832938afc99caf132b39d6a30a19bac7f2e812e28db2535b4608d27551`
- NotoEmoji-OFL.txt `79d348cb5161481f603dcda564fb129a9b2c11d7012dcad820af045fed75c020`

The license text is preserved; one trailing space is trimmed.

The face follows every existing fallback in each family. It supplies
missing monochrome emoji while preserving existing glyphs and widths.
Regional flags, keycaps and some joined sequences can still use an earlier
face selected by their first character; scalar coverage does not guarantee
composition of an entire sequence. This font does not add color emoji.
