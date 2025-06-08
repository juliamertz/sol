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

        rust-bin = pkgs.rust-bin.stable.latest;
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

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            meta.mainProgram = "newlang";
          };

          tree-sitter-grammar = pkgs.callPackage ./tree-sitter {};
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
            ])
            ++ (with pkgs; [
              gcc
            ]);
        };
      }
    );
}
