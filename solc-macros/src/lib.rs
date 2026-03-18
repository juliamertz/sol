use proc_macro::{TokenStream, TokenTree};
use quote::quote;
use syn::LitStr;

// in order to preserve newlines we need to reconstruct the source string
fn tokens_to_source(input: TokenStream) -> String {
    let mut source = String::new();
    let mut prev_end_line = None;
    let mut prev_end_col = None;

    for tt in input.into_iter() {
        let span = tt.span();
        let start_line = span.start().line();
        let start_col = span.start().column();

        if let Some(prev_line) = prev_end_line {
            let line_diff: usize = start_line.saturating_sub(prev_line);
            if line_diff > 0 {
                for _ in 0..line_diff {
                    source.push('\n');
                }
                for _ in 0..start_col {
                    source.push(' ');
                }
            } else {
                let col_diff = start_col.saturating_sub(prev_end_col.unwrap_or(0));
                if col_diff > 0 {
                    for _ in 0..col_diff {
                        source.push(' ');
                    }
                }
            }
        } else {
            for _ in 0..start_col {
                source.push(' ');
            }
        }

        match &tt {
            TokenTree::Group(group) => {
                let (open, close) = match group.delimiter() {
                    proc_macro::Delimiter::Parenthesis => ("(", ")"),
                    proc_macro::Delimiter::Brace => ("{", "}"),
                    proc_macro::Delimiter::Bracket => ("[", "]"),
                    proc_macro::Delimiter::None => ("", ""),
                };
                source.push_str(open);

                let inner = tokens_to_source(group.stream());
                source.push_str(&inner);

                let close_span = group.span_close();
                if !inner.is_empty() {
                    let open_line = group.span_open().start().line();
                    let close_line = close_span.start().line();
                    if close_line > open_line {
                        source.push('\n');
                        for _ in 0..close_span.start().column() {
                            source.push(' ');
                        }
                    }
                }
                source.push_str(close);

                prev_end_line = Some(close_span.end().line());
                prev_end_col = Some(close_span.end().column());
            }
            TokenTree::Literal(lit) => {
                source.push_str(&lit.to_string());
                prev_end_line = Some(span.end().line());
                prev_end_col = Some(span.end().column());
            }
            _ => {
                source.push_str(&tt.to_string());
                prev_end_line = Some(span.end().line());
                prev_end_col = Some(span.end().column());
            }
        }
    }

    source
}

#[proc_macro]
pub fn lex(input: TokenStream) -> TokenStream {
    let source = tokens_to_source(input);
    let lit = LitStr::new(&source, proc_macro2::Span::call_site());

    quote! {
        {
            let source: &str = #lit;
            let mut lexer = ::solc::lexer::Lexer::new(
                ::std::path::PathBuf::from("<macro>"),
                source,
            );
            let mut tokens = ::std::vec::Vec::new();
            while let Some(result) = lexer.read_token() {
                tokens.push(result.expect("failed to lex"));
            }
            tokens
        }
    }
    .into()
}

#[proc_macro]
pub fn parse(input: TokenStream) -> TokenStream {
    let source = tokens_to_source(input);
    let lit = LitStr::new(&source, proc_macro2::Span::call_site());

    quote! {
        {
            let source: &str = #lit;
            let mut parser = ::solc::parser::Parser::new(
                ::std::path::PathBuf::from("<macro>"),
                source,
            ).expect("failed to initialize parser");
            parser.parse().expect("failed to parse")
        }
    }
    .into()
}
