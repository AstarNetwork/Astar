{ nixpkgs ? import ./nixpkgs.nix { }
}:

with nixpkgs;

let
  channel = rustChannelOf { date = "2020-09-20"; channel = "nightly"; };

in rec {
  rustWasm = channel.rust.override {
    extensions = [ "rustfmt-preview" ];
    targets = [ "wasm32-unknown-unknown" ];
  };
  plasm-node = callPackage ./. { inherit rustWasm; };
}
