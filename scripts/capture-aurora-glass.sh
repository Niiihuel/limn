#!/usr/bin/env bash
# Capture Aurora's original shader over the comparison scene. No shell services run.
set -euo pipefail
reference=${1:?usage: capture-aurora-glass.sh /path/to/aurora /path/to/output}
output=${2:?usage: capture-aurora-glass.sh /path/to/aurora /path/to/output}
script_dir=$(cd -- "$(dirname -- "$0")" && pwd)
reference=$(realpath "$reference")
mkdir -p "$output"
output=$(realpath "$output")
qsb=${QSB:-qsb}
qml=${QML:-qml6}
for shader in glass blur; do
    "$qsb" --glsl "100 es,120,150" --hlsl 50 --msl 12 \
        -o "$output/$shader.frag.qsb" "$reference/shell/shaders/$shader.frag"
done
cp "$script_dir/glass-reference.qml" "$output/Reference.qml"
cd "$output"
# Offscreen Qt's software scene graph skips ShaderEffect. Use the RHI path.
QT_QUICK_BACKEND=rhi QSG_RHI_BACKEND=opengl QT_FONT_DPI=96 QT_SCALE_FACTOR=1 \
    "$qml" "$output/Reference.qml"
