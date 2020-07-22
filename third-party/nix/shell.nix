{ nixpkgs ? import ./nixpkgs.nix { }
, release ? import ./release.nix { }
}:

with nixpkgs;
with release;
with llvmPackages_latest;

stdenv.mkDerivation {
  name = "plasm-nix-shell";
  buildInputs = [ rustWasm wasm-gc zlib openssl pkgconfig ];
  LIBCLANG_PATH = "${libclang}/lib";
  # FIXME: we can remove this once prost is updated.
  PROTOC = "${protobuf}/bin/protoc";
}
