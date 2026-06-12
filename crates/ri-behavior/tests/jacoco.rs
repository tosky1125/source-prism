#![allow(missing_docs, reason = "Coverage parser test names document behavior.")]

use ri_behavior::parse_jacoco_xml;

#[test]
fn parses_jacoco_line_coverage_segments() -> Result<(), Box<dyn std::error::Error>> {
    let report = parse_jacoco_xml(
        r#"
        <report name="billing">
          <package name="com/example/billing">
            <sourcefile name="Invoice.java">
              <line nr="12" mi="0" ci="3" mb="0" cb="1"/>
              <line nr="13" mi="1" ci="0" mb="0" cb="0"/>
            </sourcefile>
          </package>
        </report>
        "#,
    )?;

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
    assert_eq!(file.file_path, "com/example/billing/Invoice.java");
    assert_eq!(first.start_line, 12);
    assert_eq!(first.hit_count, 4);
    assert_eq!(second.start_line, 13);
    assert_eq!(second.hit_count, 0);
    Ok(())
}
