{ moz_overlay ? import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz)
, nixpkgs ? import (builtins.fetchTarball https://github.com/nixos/nixpkgs-channels/archive/nixos-20.03.tar.gz)
}:

nixpkgs {
  overlays = [ moz_overlay ];
}
