let
  sources = import ./nix/sources.nix;
  rust-channel = import ./nix/rust.nix {
    inherit sources;
    targets = [ "x86_64-unknown-linux-gnu" "wasm32-unknown-unknown" ];
  };
  pkgs = import sources.nixpkgs {};
in
pkgs.mkShell {
  buildInputs = [
    pkgs.file
    pkgs.kcov
    pkgs.libressl
    pkgs.pkg-config
    pkgs.python3
    rust-channel.rust
    rust-channel.rustfmt-preview
  ];
}
