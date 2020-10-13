{ sources ? import ./sources.nix
, targets ? []
}:

let
  pkgs = import sources.nixpkgs {
    overlays = [ (import sources.nixpkgs-mozilla) ];
  };
  channel = pkgs.rustChannelOfTargets "nightly" "2020-09-15" targets;
in channel
