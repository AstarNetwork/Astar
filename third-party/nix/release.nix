{ moz_overlay ? import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz)
}:

let
  pkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
  channel = pkgs.rustChannelOf { date = "2021-02-25"; channel = "nightly"; };
in {
  inherit pkgs;
  rust-nightly = channel.rust.override {
    targets = [ "wasm32-unknown-unknown" ];
    extensions = [ "rustfmt-preview" ];
  };
}
