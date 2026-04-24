{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-filter.url = "github:numtide/nix-filter";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    nix-filter,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay) self.overlays.default];
        pkgs = import nixpkgs {inherit system overlays;};
        inherit (pkgs) lib;

        filter = nix-filter.lib;
        rust-bin = pkgs.rust-bin.nightly.latest;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust-bin.minimal;
          rustc = rust-bin.minimal;
        };

        manifest = lib.importTOML ./solc/Cargo.toml;
        solc = rustPlatform.buildRustPackage {
          pname = manifest.package.name;
          version = manifest.package.version;
          src = filter {
            root = ./.;
            include = [
              ./solc
              ./Cargo.toml
              ./Cargo.lock
            ];
          };

          nativeBuildInputs = [pkgs.makeWrapper];

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          meta.mainProgram = "solc";
        };
      in {
        packages = {
          default = solc;
          tree-sitter-grammar = pkgs.callPackage ./tree-sitter {};
          examples = let
            builders = (import ./lib.nix).mkBuilders pkgs;
            buildExample = name: builders.writeSolScript name (builtins.readFile ./examples/${name}.sol);
            examples = lib.genAttrs ["fibonacci" "list" "loop"] buildExample;
          in
            pkgs.symlinkJoin {
              name = "examples";
              paths = lib.attrValues examples;
              passthru = examples;
            };
        };

        checks = let
          inherit (self.packages.${system}) examples;
          assertOutput = {
            program,
            expected,
            message ? "",
          }:
            pkgs.runCommand "test-${program.name}" {} ''
              ${lib.getExe program} > $out
              output=$(cat $out)
              if [ "$output" != "${expected}" ]; then
                echo "ASSERTION FAILED: ${message}"
                echo "Expected:"
                echo "${expected}"
                echo "Actual:"
                echo "$output"
                exit 1
              fi
            '';
        in {
          fibonacci = assertOutput {
            program = examples.fibonacci;
            expected = "Result is 832040";
          };
          # list = assertOutput {
          #   program = examples.list;
          #   expected = "0: 10, 1: 250, 2: 450";
          # };
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
              clang-tools
              qbe
            ]);
        };
      }
    )
    // {
      overlays.default = final: prev: {
        solc = self.packages.${final.system}.default;
      };
    };
}
