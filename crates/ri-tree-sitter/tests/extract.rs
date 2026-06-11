#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_parser::{SourceFile, SymbolExtractor};
use ri_tree_sitter::TreeSitterExtractor;
use sha2::{Digest, Sha256};

#[test]
fn extracts_rust_functions_methods_and_tests() -> Result<(), Box<dyn std::error::Error>> {
    let source = r"
struct Invoice;
impl Invoice {
    fn total(&self) -> i32 { 1 }
}
#[test]
fn total_works() {}
";

    let symbols = extract(Language::Rust, "src/lib.rs", source)?;

    assert!(has_symbol(&symbols, SymbolKind::Class, "Invoice"));
    assert!(has_symbol(&symbols, SymbolKind::Method, "Invoice::total"));
    assert!(has_symbol(&symbols, SymbolKind::TestCase, "total_works"));
    Ok(())
}

#[test]
fn extracts_typescript_class_interface_and_function() -> Result<(), Box<dyn std::error::Error>> {
    let source = r"
interface Charge { total(): number }
class Invoice { total(): number { return 1 } }
function applyTax(): number { return 2 }
";

    let symbols = extract(Language::TypeScript, "src/invoice.ts", source)?;

    assert!(has_symbol(&symbols, SymbolKind::Interface, "Charge"));
    assert!(has_symbol(&symbols, SymbolKind::Class, "Invoice"));
    assert!(has_symbol(&symbols, SymbolKind::Method, "Invoice::total"));
    assert!(has_symbol(&symbols, SymbolKind::Function, "applyTax"));
    Ok(())
}

#[test]
fn extracts_python_class_and_function() -> Result<(), Box<dyn std::error::Error>> {
    let source = r"
class Invoice:
    def total(self):
        return 1

def apply_tax():
    return 2
";

    let symbols = extract(Language::Python, "invoice.py", source)?;

    assert!(has_symbol(&symbols, SymbolKind::Class, "Invoice"));
    assert!(has_symbol(&symbols, SymbolKind::Method, "Invoice::total"));
    assert!(has_symbol(&symbols, SymbolKind::Function, "apply_tax"));
    Ok(())
}

#[test]
fn rust_function_inside_module_remains_function() -> Result<(), Box<dyn std::error::Error>> {
    let source = r"
mod billing {
    fn apply_tax() -> i32 { 1 }
}
";

    let symbols = extract(Language::Rust, "src/lib.rs", source)?;

    assert!(has_symbol(
        &symbols,
        SymbolKind::Function,
        "billing::apply_tax"
    ));
    Ok(())
}

#[test]
fn extracts_go_function_and_method() -> Result<(), Box<dyn std::error::Error>> {
    let source = r"
package invoice
type Invoice struct {}
func (i *Invoice) Total() int { return 1 }
func ApplyTax() int { return 2 }
";

    let symbols = extract(Language::Go, "invoice.go", source)?;

    assert!(has_symbol(&symbols, SymbolKind::Method, "Invoice.Total"));
    assert!(has_symbol(&symbols, SymbolKind::Function, "ApplyTax"));
    Ok(())
}

fn extract(
    language: Language,
    path: &str,
    source: &str,
) -> Result<Vec<ri_symbols::SymbolRecord>, Box<dyn std::error::Error>> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    let path = FilePath::new(path)?;
    let hash = content_hash(source);
    let file = SourceFile::new(repo, commit, path, language, hash.as_str(), source);
    Ok(TreeSitterExtractor::new().extract_symbols(&file)?)
}

fn has_symbol(symbols: &[ri_symbols::SymbolRecord], kind: SymbolKind, fqn: &str) -> bool {
    symbols
        .iter()
        .any(|symbol| symbol.kind == kind && symbol.fqn == fqn)
}

fn content_hash(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    format!("{:x}", hasher.finalize())
}
