#!/bin/sh
# Prints what a screen reader on Linux is given by a program, read from AT-SPI
# on an accessibility bus of its own, so it works on a desktop that has none.
#
#   nix-shell --run "nix-shell -p at-spi2-core dbus gobject-introspection \
#     'python3.withPackages (p: [p.pygobject3])' --run \
#     'dbus-run-session -- tools/atspi/run.sh target/debug/limn-gallery --page switch'"
#
# The program's window opens for a few seconds.
set -eu
libexec=$(dirname "$(dirname "$(readlink -f "$(command -v at-spi2-registryd 2>/dev/null || true)")")")/libexec
[ -x "$libexec/at-spi-bus-launcher" ] || libexec=$(nix-build '<nixpkgs>' -A at-spi2-core --no-out-link)/libexec
"$libexec/at-spi-bus-launcher" --launch-immediately &
launcher=$!
sleep 1
"$libexec/at-spi2-registryd" &
registry=$!
sleep 1
# AccessKit publishes nothing until something asks for it.
busctl --user set-property org.a11y.Bus /org/a11y/bus org.a11y.Status IsEnabled b true
busctl --user set-property org.a11y.Bus /org/a11y/bus org.a11y.Status ScreenReaderEnabled b true
"$@" >/dev/null 2>&1 &
program=$!
sleep 4
python3 "$(dirname "$0")/dump.py" 2>/dev/null || true
kill "$program" "$registry" "$launcher" 2>/dev/null || true
