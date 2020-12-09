{ release ? import ./release.nix { }
}:

with release.pkgs;
with llvmPackages;

stdenv.mkDerivation {
  name = "plasm-nix-shell";
  nativeBuildInputs = [ clang ];
  buildInputs = [
    release.rust-nightly
    pkg-config
    openssl
    cmake
  ] ++ stdenv.lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
  ];
  ROCKSDB_LIB_DIR = "${rocksdb}/lib";
  LIBCLANG_PATH = "${libclang}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
