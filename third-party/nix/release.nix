{ nixpkgs ? import ./nixpkgs.nix { }
}:

with nixpkgs;

let
  channel = rustChannelOf { date = "2020-07-10"; channel = "nightly"; };

in rec {
  rustWasm = channel.rust.override {
    targets = [ "wasm32-unknown-unknown" ];
    extensions = [ "rustfmt-preview" ];
  };
  plasm-node = callPackage ./. { inherit rustWasm; };
}
