{ sources ? import ./sources.nix
, targets ? []
}:

let
  pkgs = import sources.nixpkgs {
    overlays = [ (import sources.nixpkgs-mozilla) ];
  };
  channel = pkgs.rustChannelOf {
    channel = "nightly";
    date = "2020-05-15";
    inherit targets;
  };
in channel
