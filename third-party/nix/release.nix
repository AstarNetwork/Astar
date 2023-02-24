{ rust-overlay ? import (builtins.fetchTarball https://github.com/oxalica/rust-overlay/archive/master.tar.gz)
}:

let
  pkgs = import <nixpkgs> { overlays = [ rust-overlay ]; };
in {
  inherit pkgs;
  rust-nightly = pkgs.rust-bin.fromRustupToolchainFile ../../rust-toolchain.toml;
}
