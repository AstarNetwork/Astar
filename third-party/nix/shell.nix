{ release ? import ./release.nix { }
}:

with release.pkgs;
with llvmPackages;

mkShell {
  nativeBuildInputs = [ clang ];
  buildInputs = [
    release.rust-nightly
    zlib
  ] ++ stdenv.lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
  ];
  ROCKSDB_LIB_DIR = "${rocksdb}/lib";
  LIBCLANG_PATH = "${libclang}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
