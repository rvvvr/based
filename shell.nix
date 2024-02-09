let
  rust-overlay = builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz";

  pkgs = import <nixpkgs> {
    overlays = [ (import rust-overlay) ];
  };
in
pkgs.mkShell rec {
  buildInputs = with pkgs; [
    rustup
    wayland
    wgpu-utils
    vulkan-loader
    libxkbcommon
    fontconfig
  ];
  nativeBuildInputs = with pkgs; [
    pkg-config 
  ];

  shellHook = ''
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${builtins.toString (pkgs.lib.makeLibraryPath buildInputs)}";
  '';
}
