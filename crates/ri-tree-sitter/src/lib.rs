#![allow(
    missing_docs,
    reason = "Tree-sitter adapter API is internal milestone surface."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "Tree-sitter grammar crates currently depend on adjacent tree-sitter versions."
)]

mod calls;
mod names;
mod traversal;

use ri_core::Language;
use ri_parser::{CallExtractor, CallReference, ParserError, SourceFile, SymbolExtractor};
use ri_symbols::SymbolRecord;
use std::path::Path;
use tree_sitter::Parser;

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct TreeSitterExtractor;

impl TreeSitterExtractor {
    pub const fn new() -> Self {
        Self
    }
}

impl SymbolExtractor for TreeSitterExtractor {
    fn extract_symbols(&self, file: &SourceFile<'_>) -> Result<Vec<SymbolRecord>, ParserError> {
        let tree = parse_tree(file)?;
        Ok(traversal::extract_tree_symbols(file, tree.root_node()))
    }
}

impl CallExtractor for TreeSitterExtractor {
    fn extract_calls(&self, file: &SourceFile<'_>) -> Result<Vec<CallReference>, ParserError> {
        let tree = parse_tree(file)?;
        Ok(calls::extract_tree_calls(file, tree.root_node()))
    }
}

fn parse_tree(file: &SourceFile<'_>) -> Result<tree_sitter::Tree, ParserError> {
    let mut parser = Parser::new();
    set_language(&mut parser, file)?;
    let tree = parser
        .parse(file.source, None)
        .ok_or_else(|| ParserError::ParseFailed {
            path: file.path.to_string(),
            message: "tree-sitter returned no tree".to_owned(),
        })?;
    if tree.root_node().has_error() {
        return Err(ParserError::ParseFailed {
            path: file.path.to_string(),
            message: "syntax tree contains errors".to_owned(),
        });
    }
    Ok(tree)
}

fn set_language(parser: &mut Parser, file: &SourceFile<'_>) -> Result<(), ParserError> {
    match file.language {
        Language::Rust => parser.set_language(&tree_sitter_rust::LANGUAGE.into()),
        Language::TypeScript | Language::JavaScript if is_jsx_path(file.path.as_str()) => {
            parser.set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
        }
        Language::TypeScript | Language::JavaScript => {
            parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        }
        Language::Python => parser.set_language(&tree_sitter_python::LANGUAGE.into()),
        Language::Go => parser.set_language(&tree_sitter_go::LANGUAGE.into()),
        other => {
            return Err(ParserError::UnsupportedLanguage { language: other });
        }
    }
    .map_err(|error| ParserError::ParseFailed {
        path: "<language>".to_owned(),
        message: error.to_string(),
    })
}

fn is_jsx_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("tsx") || extension.eq_ignore_ascii_case("jsx")
        })
}
