{ release ? import ./release.nix { }
}:

with release.pkgs;
with llvmPackages_latest;

stdenv.mkDerivation {
  name = "plasm-nix-shell";
  buildInputs = [
    clang
    cmake
    pkg-config
    release.rust-nightly
  ] ++ stdenv.lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
  ];
  ROCKSDB_LIB_DIR = "${rocksdb}/lib";
  LIBCLANG_PATH = "${libclang}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
