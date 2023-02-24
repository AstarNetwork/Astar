{ rust-overlay ? import (builtins.fetchTarball https://github.com/oxalica/rust-overlay/archive/master.tar.gz)
, nixpkgs ? import (builtins.fetchTarball https://github.com/nixos/nixpkgs-channels/archive/nixos-21.11.tar.gz)
}:

nixpkgs {
  overlays = [ rust-overlay ];
}
