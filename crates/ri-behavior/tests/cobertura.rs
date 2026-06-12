#![allow(missing_docs, reason = "Coverage parser test names document behavior.")]

use ri_behavior::parse_cobertura_xml;

#[test]
fn parses_cobertura_line_coverage_segments() -> Result<(), Box<dyn std::error::Error>> {
    let report = parse_cobertura_xml(
        r#"
        <coverage>
          <packages>
            <package name="billing">
              <classes>
                <class filename="src/invoice.rs">
                  <lines>
                    <line number="3" hits="1"/>
                    <line number="4" hits="0"/>
                  </lines>
                </class>
              </classes>
            </package>
          </packages>
        </coverage>
        "#,
    )?;

    assert_eq!(report.segment_count(), 2);
    let file = report
        .files
        .first()
        .ok_or_else(|| std::io::Error::other("missing coverage file"))?;
    assert_eq!(file.file_path, "src/invoice.rs");
    let first = file
        .segments
        .first()
        .ok_or_else(|| std::io::Error::other("missing first segment"))?;
    let second = file
        .segments
        .get(1)
        .ok_or_else(|| std::io::Error::other("missing second segment"))?;
    assert_eq!(first.start_line, 3);
    assert_eq!(first.hit_count, 1);
    assert_eq!(second.start_line, 4);
    assert_eq!(second.hit_count, 0);
    Ok(())
}
