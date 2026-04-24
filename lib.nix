let
  mkBuilders = pkgs: {
    writeSolScript = name: source: let
      sourceFile = pkgs.writeText "source" source;
    in
      pkgs.runCommand name {
        nativeBuildInputs = [pkgs.qbe pkgs.gcc pkgs.solc];
        meta.mainProgram = name;
      } # sh
      ''
        mkdir -p "$out/bin"
        solc build --outdir . "${sourceFile}"
        install --mode=+x a.out $out/bin/${name}
      '';
  };
in {
  inherit mkBuilders;
}
