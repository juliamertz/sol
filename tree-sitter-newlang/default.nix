{
  tree-sitter,
  ...
}:
tree-sitter.buildGrammar {
  src = ./.;
  version = "0.1.0";
  language = "newlang";
}
