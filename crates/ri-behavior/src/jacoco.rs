use serde::Deserialize;

use crate::{BehaviorError, CoverageFile, CoverageReport, CoverageSegment};

pub fn parse_jacoco_xml(xml: &str) -> Result<CoverageReport, BehaviorError> {
    let raw =
        quick_xml::de::from_str::<RawReport>(xml).map_err(|error| BehaviorError::JacocoXml {
            message: error.to_string(),
        })?;
    let files = raw
        .packages
        .iter()
        .flat_map(coverage_files_for_package)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CoverageReport { files })
}

#[derive(Debug, Default, Deserialize)]
struct RawReport {
    #[serde(rename = "package", default)]
    packages: Vec<RawPackage>,
}

#[derive(Debug, Deserialize)]
struct RawPackage {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "sourcefile", default)]
    source_files: Vec<RawSourceFile>,
}

#[derive(Debug, Deserialize)]
struct RawSourceFile {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "line", default)]
    lines: Vec<RawLine>,
}

#[derive(Debug, Deserialize)]
struct RawLine {
    #[serde(rename = "@nr")]
    number: String,
    #[serde(rename = "@ci")]
    covered_instructions: String,
    #[serde(rename = "@cb")]
    covered_branches: String,
}

fn coverage_files_for_package(package: &RawPackage) -> Vec<Result<CoverageFile, BehaviorError>> {
    package
        .source_files
        .iter()
        .map(|file| coverage_file_from_raw(package.name.as_str(), file))
        .collect()
}

fn coverage_file_from_raw(
    package_name: &str,
    raw: &RawSourceFile,
) -> Result<CoverageFile, BehaviorError> {
    let segments = raw
        .lines
        .iter()
        .map(segment_from_raw)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CoverageFile {
        file_path: source_path(package_name, &raw.name),
        segments,
    })
}

fn segment_from_raw(raw: &RawLine) -> Result<CoverageSegment, BehaviorError> {
    let start_line = parse_u32(&raw.number, "line")?;
    let hit_count = parse_u32(&raw.covered_instructions, "covered_instructions")?
        .saturating_add(parse_u32(&raw.covered_branches, "covered_branches")?);
    Ok(CoverageSegment {
        start_line,
        end_line: start_line,
        hit_count,
    })
}

fn source_path(package_name: &str, file_name: &str) -> String {
    if package_name.is_empty() {
        return file_name.to_owned();
    }
    format!("{package_name}/{file_name}")
}

fn parse_u32(value: &str, field: &'static str) -> Result<u32, BehaviorError> {
    value
        .parse::<u32>()
        .map_err(|error| BehaviorError::JacocoXml {
            message: format!("invalid {field}: {error}"),
        })
}
