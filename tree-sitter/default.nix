{
  stdenv,
  tree-sitter,
  nodejs,
}: let
  sourceDir = stdenv.mkDerivation {
    name = "grammar-source";
    src = ./.;
    nativeBuildInputs = [
      tree-sitter
      nodejs
    ];

    buildPhase = ''
      tree-sitter generate
    '';

    installPhase = ''
      mkdir -p $out
      cp -r * $out
    '';
  };
in
  tree-sitter.buildGrammar {
    src = sourceDir;
    version = "0.1.0";
    language = "sol";
  }
