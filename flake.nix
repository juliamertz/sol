{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {inherit system overlays;};
        inherit (pkgs) lib;

        rust-bin = pkgs.rust-bin.nightly.latest;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust-bin.minimal;
          rustc = rust-bin.minimal;
        };

        manifest = lib.importTOML ./Cargo.toml;
      in {
        packages = {
          default = rustPlatform.buildRustPackage rec {
            pname = manifest.package.name;
            version = manifest.package.version;
            src = ./.;

            nativeBuildInputs = [pkgs.makeWrapper];
            buildInputs = [
              pkgs.nix
              pkgs.nixfmt-rfc-style
            ];

            postInstall = ''
              wrapProgram "$out/bin/nixpins" --set PATH "${lib.makeBinPath buildInputs}"
            '';

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            meta.mainProgram = "nixpins";
          };
        };

        devShells.default = pkgs.mkShell {
          packages =
            self.packages.${system}.default.buildInputs
            ++ (with rust-bin; [
              (minimal.override {
                extensions = [
                  "clippy"
                  "rust-src"
                ];
              })

              rustfmt
              rust-analyzer
            ]);
        };
      }
    );
}

