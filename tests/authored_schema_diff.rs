#![cfg(feature = "authored-schema")]

use upgrade::{AuthoredSchemaIdentity, AuthoredSchemaSnapshot};

#[test]
fn authored_schema_files_feed_deterministic_schema_difference() {
    let old = AuthoredSchemaSnapshot::from_schema_file(
        "tests/fixtures/schema_diff/old.schema",
        AuthoredSchemaIdentity::new("representative", "0.1.0"),
    )
    .expect("old authored schema adapts");
    let new = AuthoredSchemaSnapshot::from_schema_file(
        "tests/fixtures/schema_diff/new.schema",
        AuthoredSchemaIdentity::new("representative", "0.1.1"),
    )
    .expect("new authored schema adapts");

    assert_eq!(old.missing_facts(), &[]);
    assert_eq!(new.missing_facts(), &[]);

    let report = new.snapshot().difference_from(old.snapshot());
    let stable_text = report.stable_text();

    assert_eq!(
        stable_text,
        new.snapshot().difference_from(old.snapshot()).stable_text()
    );
    assert!(stable_text.contains(
        "added-field family=AccountFamily field=created_at | needs explicit upgrade rule"
    ));
    assert!(
        stable_text.contains("likely-renamed-field family=AccountFamily old=display_name new=name")
    );
    assert!(stable_text.contains(
        "removed-field family=AccountFamily field=legacy_note | needs explicit upgrade rule"
    ));
    assert!(
        stable_text.contains(
            "type-changed family=AccountFamily field=quota | needs explicit upgrade rule"
        )
    );
    assert!(
        stable_text
            .contains("family-hash-changed family=AccountFamily | needs explicit upgrade rule")
    );
}

#[test]
fn authored_schema_reports_missing_record_field_facts() {
    let source = r#"
[]
[]
{
  Event [Created Deleted]
  EventFamily (Family { record.Event table.events key.Domain })
}
"#;

    let snapshot = AuthoredSchemaSnapshot::from_schema_text(
        source,
        AuthoredSchemaIdentity::new("representative", "0.1.0"),
    )
    .expect("authored schema with enum family adapts with diagnostics");

    assert_eq!(
        snapshot.missing_facts()[0].stable_text(),
        "missing family-record-fields family=EventFamily record=Event declaration-kind=enum"
    );
}
