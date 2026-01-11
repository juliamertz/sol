{
  stdenv,
  tree-sitter,
  nodejs,
}: let
  src = stdenv.mkDerivation {
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
    inherit src;
    version = "0.1.0";
    language = "sol";
  }
