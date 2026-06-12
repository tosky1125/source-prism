#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_behavior::parse_lcov;

#[test]
fn parses_lcov_line_coverage_segments() -> Result<(), Box<dyn std::error::Error>> {
    let lcov = r"
TN:
SF:src/invoice.rs
DA:3,1
DA:4,0
end_of_record
";

    let report = parse_lcov(lcov)?;

    assert_eq!(report.segment_count(), 2);
    let file = report
        .files
        .first()
        .ok_or_else(|| std::io::Error::other("missing coverage file"))?;
    let first = file
        .segments
        .first()
        .ok_or_else(|| std::io::Error::other("missing first segment"))?;
    let second = file
        .segments
        .get(1)
        .ok_or_else(|| std::io::Error::other("missing second segment"))?;
    assert_eq!(file.file_path, "src/invoice.rs");
    assert_eq!(first.start_line, 3);
    assert_eq!(first.hit_count, 1);
    assert_eq!(second.hit_count, 0);
    Ok(())
}
