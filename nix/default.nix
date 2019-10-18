{ nixpkgs ? import ./nixpkgs.nix { }
, rustWasm
}:

with nixpkgs;
with llvmPackages_latest;

rustPlatform.buildRustPackage rec {
  name = "plasm-node";
  src = ./..;
  cargoSha256 = null; 
  buildInputs = [ rustWasm wasm-gc pkgconfig openssl clang ];
  LIBCLANG_PATH = "${libclang}/lib";
  # FIXME: we can remove this once prost is updated.
  PROTOC = "${protobuf}/bin/protoc";
}
