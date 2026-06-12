use serde::{Deserialize, Serialize};

use crate::BehaviorError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CoverageReport {
    pub files: Vec<CoverageFile>,
}

impl CoverageReport {
    pub fn segment_count(&self) -> u32 {
        u32::try_from(
            self.files
                .iter()
                .map(|file| file.segments.len())
                .sum::<usize>(),
        )
        .unwrap_or(u32::MAX)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CoverageFile {
    pub file_path: String,
    pub segments: Vec<CoverageSegment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CoverageSegment {
    pub start_line: u32,
    pub end_line: u32,
    pub hit_count: u32,
}

pub fn parse_lcov(input: &str) -> Result<CoverageReport, BehaviorError> {
    let mut files = Vec::new();
    let mut current = CurrentFile::default();
    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line == "TN:" {
            continue;
        }
        if let Some(path) = line.strip_prefix("SF:") {
            current.finish_into(&mut files);
            current.file_path = Some(path.to_owned());
            continue;
        }
        if let Some(data) = line.strip_prefix("DA:") {
            current.segments.push(parse_da(data)?);
            continue;
        }
        if line == "end_of_record" {
            current.finish_into(&mut files);
        }
    }
    current.finish_into(&mut files);
    Ok(CoverageReport { files })
}

#[derive(Default)]
struct CurrentFile {
    file_path: Option<String>,
    segments: Vec<CoverageSegment>,
}

impl CurrentFile {
    fn finish_into(&mut self, files: &mut Vec<CoverageFile>) {
        let Some(file_path) = self.file_path.take() else {
            self.segments.clear();
            return;
        };
        files.push(CoverageFile {
            file_path,
            segments: std::mem::take(&mut self.segments),
        });
    }
}

fn parse_da(data: &str) -> Result<CoverageSegment, BehaviorError> {
    let (line, count) = data.split_once(',').ok_or_else(|| BehaviorError::Lcov {
        message: format!("invalid DA record: {data}"),
    })?;
    let start_line = parse_u32(line, "line")?;
    let hit_count = parse_u32(count, "hit_count")?;
    Ok(CoverageSegment {
        start_line,
        end_line: start_line,
        hit_count,
    })
}

fn parse_u32(value: &str, field: &'static str) -> Result<u32, BehaviorError> {
    value.parse::<u32>().map_err(|error| BehaviorError::Lcov {
        message: format!("invalid {field}: {error}"),
    })
}
