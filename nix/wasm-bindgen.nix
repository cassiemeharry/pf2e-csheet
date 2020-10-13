{ sources ? import ./sources.nix
, pkgs ? import sources.nixpkgs {}
}:

let
  rust-system = import ./rust.nix { inherit sources; targets = []; };
  naersk-system = pkgs.callPackage sources.naersk {
    rustc = rust-system;
    cargo = rust-system;
  };
  wasm-bindgen-locked-source = pkgs.stdenv.mkDerivation {
    # This whole derivation is only necessary because wasm-bindgen doens't
    # provide a Cargo.lock. I've generated it manually and vendored it alongside
    # this file.
    name = "wasm-bindgen-locked-source";
    version = "0.2.68";
    buildInputs = [ pkgs.stdenv ];
    src = pkgs.fetchFromGitHub {
      owner = "rustwasm";
      repo = "wasm-bindgen";
      rev = "0.2.68";
      sha256 = "0110i8c295dkynd6v1x47h1j9smbqb3pj8ccpyw5sy25yzd3v9ng";
    };
    buildPhase = ''
      runHook preBuild
      set -x
      cp "${./wasm-bindgen-v0.2.68-Cargo.lock}" Cargo.lock
      set +x
      runHook postBuild
    '';
    installPhase = ''
      runHook preInstall
      cp -R . "$out"
      runHook postInstall
    '';
  };
in
  naersk-system.buildPackage {
    name = "wasm-bindgen";
    version = "0.2.68";
    src = wasm-bindgen-locked-source;
    buildInputs = [ pkgs.libressl pkgs.pkg-config ];
    cargoBuildOptions = orig:
      orig ++ [ "--package" "wasm-bindgen-cli" ];
    compressTarget = false;
    singleStep = true;
  }
