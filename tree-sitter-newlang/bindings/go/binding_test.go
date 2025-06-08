package tree_sitter_newlang_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_newlang "github.com/tree-sitter/tree-sitter-newlang/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_newlang.Language())
	if language == nil {
		t.Errorf("Error loading Newlang grammar")
	}
}
