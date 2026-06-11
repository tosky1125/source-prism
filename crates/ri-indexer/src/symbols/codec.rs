use ri_core::{FilePath, Language, SymbolId, SymbolKind};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
use sqlx::Row as _;

use super::SymbolStoreError;

pub(super) fn symbol_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<SymbolRecord, SymbolStoreError> {
    let name = row.try_get::<String, _>("name")?;
    let language = language_from_id(row.try_get::<String, _>("language")?.as_str())?;
    let kind = kind_from_id(row.try_get::<String, _>("kind")?.as_str())?;
    let fqn = row
        .try_get::<Option<String>, _>("fqn")?
        .unwrap_or_else(|| name.clone());
    let range = SymbolRange::new(
        stored_range_value(row, "start_line")?,
        stored_range_value(row, "start_col")?,
        stored_range_value(row, "end_line")?,
        stored_range_value(row, "end_col")?,
    );
    Ok(SymbolRecord::from_ids(
        SymbolId::new(row.try_get::<String, _>("stable_symbol_id")?)?,
        SymbolId::new(row.try_get::<String, _>("symbol_id")?)?,
        FilePath::new(row.try_get::<String, _>("file_path")?)?,
        SymbolSpec::new(language, kind, name, fqn, range),
    ))
}

pub(super) fn range_value(value: u32, field: &'static str) -> Result<i32, SymbolStoreError> {
    i32::try_from(value).map_err(|_| SymbolStoreError::InvalidRangeValue { field, value })
}

fn stored_range_value(
    row: &sqlx::postgres::PgRow,
    field: &'static str,
) -> Result<u32, SymbolStoreError> {
    let value = row.try_get::<i32, _>(field)?;
    u32::try_from(value).map_err(|_| SymbolStoreError::InvalidStoredRange { field, value })
}

pub(super) const fn language_id(language: Language) -> &'static str {
    match language {
        Language::TypeScript => "typescript",
        Language::JavaScript => "javascript",
        Language::Php => "php",
        Language::Python => "python",
        Language::Java => "java",
        Language::Go => "go",
        Language::Rust => "rust",
        _ => "unknown",
    }
}

fn language_from_id(value: &str) -> Result<Language, SymbolStoreError> {
    match value {
        "typescript" => Ok(Language::TypeScript),
        "javascript" => Ok(Language::JavaScript),
        "php" => Ok(Language::Php),
        "python" => Ok(Language::Python),
        "java" => Ok(Language::Java),
        "go" => Ok(Language::Go),
        "rust" => Ok(Language::Rust),
        "unknown" => Ok(Language::Unknown),
        other => Err(SymbolStoreError::InvalidStoredEnum {
            field: "language",
            value: other.to_owned(),
        }),
    }
}

pub(super) const fn kind_id(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Module => "module",
        SymbolKind::Class => "class",
        SymbolKind::Interface => "interface",
        SymbolKind::Enum => "enum",
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Field => "field",
        SymbolKind::RouteHandler => "route_handler",
        SymbolKind::TestCase => "test_case",
        _ => "unknown",
    }
}

fn kind_from_id(value: &str) -> Result<SymbolKind, SymbolStoreError> {
    match value {
        "module" => Ok(SymbolKind::Module),
        "class" => Ok(SymbolKind::Class),
        "interface" => Ok(SymbolKind::Interface),
        "enum" => Ok(SymbolKind::Enum),
        "function" => Ok(SymbolKind::Function),
        "method" => Ok(SymbolKind::Method),
        "constructor" => Ok(SymbolKind::Constructor),
        "field" => Ok(SymbolKind::Field),
        "route_handler" => Ok(SymbolKind::RouteHandler),
        "test_case" => Ok(SymbolKind::TestCase),
        other => Err(SymbolStoreError::InvalidStoredEnum {
            field: "kind",
            value: other.to_owned(),
        }),
    }
}
