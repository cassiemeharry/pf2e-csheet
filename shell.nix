let
  sources = import ./nix/sources.nix;
  rust-channel = import ./nix/rust.nix {
    inherit sources;
    targets = [ "x86_64-unknown-linux-gnu" "wasm32-unknown-unknown" ];
  };
  pkgs = import sources.nixpkgs {};
  wasm-bindgen = import ./nix/wasm-bindgen.nix { inherit sources pkgs; };
in
pkgs.mkShell {
  buildInputs = [
    pkgs.file
    pkgs.gdb
    pkgs.kcov
    pkgs.libressl
    pkgs.pkg-config
    pkgs.python3
    rust-channel
    # rust-channel.rustfmt-preview
    wasm-bindgen
  ];

  LIBCLANG_PATH="${pkgs.llvmPackages.libclang}/lib";
}
