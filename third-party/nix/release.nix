{ nixpkgs ? import ./nixpkgs.nix { }
}:

with nixpkgs;

let
  channel = rustChannelOf { date = "2020-05-15"; channel = "nightly"; };

in rec {
  rustWasm = channel.rust.override {
    targets = [ "wasm32-unknown-unknown" ];
  };
  plasm-node = callPackage ./. { inherit rustWasm; };
}
