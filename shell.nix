{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [ stdenv.cc pkg-config dioxus-cli ];
  buildInputs = with pkgs; [ wayland libxkbcommon ];
  # winit and wgpu dlopen these at runtime; Cisco's OpenH264, which limn-video loads, links libstdc++.
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [ wayland libxkbcommon vulkan-loader libGL stdenv.cc.cc.lib ]);
}
