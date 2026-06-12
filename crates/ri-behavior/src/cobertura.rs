use serde::Deserialize;

use crate::{BehaviorError, CoverageFile, CoverageReport, CoverageSegment};

pub fn parse_cobertura_xml(xml: &str) -> Result<CoverageReport, BehaviorError> {
    let raw = quick_xml::de::from_str::<RawCoverage>(xml).map_err(|error| {
        BehaviorError::CoberturaXml {
            message: error.to_string(),
        }
    })?;
    let files = raw
        .packages
        .packages
        .into_iter()
        .flat_map(|package| package.classes.classes)
        .map(coverage_file_from_raw)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CoverageReport { files })
}

#[derive(Debug, Default, Deserialize)]
struct RawCoverage {
    #[serde(default)]
    packages: RawPackages,
}

#[derive(Debug, Default, Deserialize)]
struct RawPackages {
    #[serde(rename = "package", default)]
    packages: Vec<RawPackage>,
}

#[derive(Debug, Default, Deserialize)]
struct RawPackage {
    #[serde(default)]
    classes: RawClasses,
}

#[derive(Debug, Default, Deserialize)]
struct RawClasses {
    #[serde(rename = "class", default)]
    classes: Vec<RawClass>,
}

#[derive(Debug, Default, Deserialize)]
struct RawClass {
    #[serde(rename = "@filename")]
    filename: String,
    #[serde(default)]
    lines: RawLines,
}

#[derive(Debug, Default, Deserialize)]
struct RawLines {
    #[serde(rename = "line", default)]
    lines: Vec<RawLine>,
}

#[derive(Debug, Deserialize)]
struct RawLine {
    #[serde(rename = "@number")]
    number: String,
    #[serde(rename = "@hits")]
    hits: String,
}

fn coverage_file_from_raw(raw: RawClass) -> Result<CoverageFile, BehaviorError> {
    let segments = raw
        .lines
        .lines
        .iter()
        .map(segment_from_raw)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CoverageFile {
        file_path: raw.filename,
        segments,
    })
}

fn segment_from_raw(raw: &RawLine) -> Result<CoverageSegment, BehaviorError> {
    let start_line = parse_u32(&raw.number, "line")?;
    Ok(CoverageSegment {
        start_line,
        end_line: start_line,
        hit_count: parse_u32(&raw.hits, "hits")?,
    })
}

fn parse_u32(value: &str, field: &'static str) -> Result<u32, BehaviorError> {
    value
        .parse::<u32>()
        .map_err(|error| BehaviorError::CoberturaXml {
            message: format!("invalid {field}: {error}"),
        })
}
