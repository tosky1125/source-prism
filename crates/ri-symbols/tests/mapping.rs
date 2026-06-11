#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec, innermost_symbol_for_line};

#[test]
fn innermost_symbol_wins_when_changed_line_is_nested() -> Result<(), Box<dyn std::error::Error>> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    let file_path = FilePath::new("src/lib.rs")?;
    let outer = symbol(&repo, &commit, file_path.clone(), "outer", 1, 10);
    let inner = symbol(&repo, &commit, file_path, "outer::inner", 4, 6);

    let symbols = vec![outer, inner];
    let found = innermost_symbol_for_line(&symbols, 5);

    assert_eq!(
        found.map(|symbol| symbol.fqn.as_str()),
        Some("outer::inner")
    );
    Ok(())
}

fn symbol(
    repo: &RepoId,
    commit: &CommitSha,
    file_path: FilePath,
    fqn: &str,
    start_line: u32,
    end_line: u32,
) -> SymbolRecord {
    SymbolRecord::new(
        repo,
        commit,
        file_path,
        "hash",
        SymbolSpec::new(
            Language::Rust,
            SymbolKind::Function,
            fqn.rsplit("::").next().unwrap_or(fqn),
            fqn,
            SymbolRange::new(start_line, 0, end_line, 1),
        ),
    )
}
