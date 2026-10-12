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
