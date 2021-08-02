{ release ? import ./release.nix { }
}:

with release.pkgs;
with llvmPackages;

mkShell {
  nativeBuildInputs = [ clang ];
  buildInputs = [
    release.rust-nightly
    zlib
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
  ];
  ROCKSDB_LIB_DIR = "${rocksdb}/lib";
  LIBCLANG_PATH = "${libclang.lib}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
