{ nixpkgs ? import ./nixpkgs.nix { }
}:

with nixpkgs;

let
  channel = rustChannelOf { date = "2020-08-20"; channel = "nightly"; };

in rec {
  rustWasm = channel.rust.override {
    targets = [ "wasm32-unknown-unknown" ];
    extensions = [ "rustfmt-preview" ];
  };
  plasm-node = callPackage ./. { inherit rustWasm; };
}
