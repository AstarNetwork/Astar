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
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
  ];
  LIBCLANG_PATH = "${clang-unwrapped.lib}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
