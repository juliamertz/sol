import XCTest
import SwiftTreeSitter
import TreeSitterNewlang

final class TreeSitterNewlangTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_newlang())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading Newlang grammar")
    }
}
