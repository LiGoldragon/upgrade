use upgrade::{
    ChangeClassification, FamilyIdentity, FamilySchema, FamilySchemaHash, FieldIdentity,
    FieldSchema, FieldType, KeyIdentity, SchemaChangeKind, SemaSchemaSnapshot, StorageIdentity,
};

#[test]
fn schema_difference_report_is_deterministic_and_classified() {
    let old = SemaSchemaSnapshot::new([
        family(
            "Account",
            "accounts",
            ["identifier"],
            "hash-account-old",
            [
                ("identifier", "Text"),
                ("display_name", "Text"),
                ("quota", "Unsigned64"),
                ("legacy_note", "Text"),
                ("state", "Text"),
            ],
        ),
        family(
            "AuditLog",
            "audit_log",
            ["identifier"],
            "hash-audit",
            [("identifier", "Text"), ("message", "Text")],
        ),
        family(
            "OldSession",
            "sessions",
            ["identifier"],
            "hash-session-old",
            [("identifier", "Text"), ("token", "Bytes")],
        ),
        family(
            "RemovedOnly",
            "removed_only",
            ["identifier"],
            "hash-removed",
            [("identifier", "Text")],
        ),
        family(
            "StorageMoved",
            "storage_old",
            ["identifier"],
            "hash-storage-old",
            [("identifier", "Text")],
        ),
        family(
            "KeyChanged",
            "key_changed",
            ["identifier"],
            "hash-key-old",
            [("identifier", "Text"), ("scope", "Text")],
        ),
    ]);

    let new = SemaSchemaSnapshot::new([
        family(
            "Account",
            "accounts",
            ["identifier"],
            "hash-account-new",
            [
                ("identifier", "Text"),
                ("name", "Text"),
                ("quota", "Signed64"),
                ("state", "Text"),
                ("created_at", "Timestamp"),
            ],
        ),
        family(
            "CurrentSession",
            "sessions",
            ["identifier"],
            "hash-session-new",
            [("identifier", "Text"), ("token", "Bytes")],
        ),
        family(
            "AddedOnly",
            "added_only",
            ["identifier"],
            "hash-added",
            [("identifier", "Text")],
        ),
        family(
            "StorageMoved",
            "storage_new",
            ["identifier"],
            "hash-storage-new",
            [("identifier", "Text")],
        ),
        family(
            "KeyChanged",
            "key_changed",
            ["identifier", "scope"],
            "hash-key-new",
            [("identifier", "Text"), ("scope", "Text")],
        ),
    ]);

    let report = new.difference_from(&old);
    let stable_text = report.stable_text();

    assert_eq!(stable_text, new.difference_from(&old).stable_text());
    assert!(stable_text.contains("added-family family=AddedOnly | auto-safe"));
    assert!(stable_text.contains("removed-family family=AuditLog | unsupported"));
    assert!(stable_text.contains("removed-family family=RemovedOnly | unsupported"));
    assert!(stable_text.contains(
        "likely-renamed-family old=OldSession new=CurrentSession | needs explicit upgrade rule"
    ));
    assert!(
        stable_text
            .contains("added-field family=Account field=created_at | needs explicit upgrade rule")
    );
    assert!(
        stable_text.contains(
            "removed-field family=Account field=legacy_note | needs explicit upgrade rule"
        )
    );
    assert!(stable_text.contains("likely-renamed-field family=Account old=display_name new=name | needs explicit upgrade rule"));
    assert!(
        stable_text
            .contains("type-changed family=Account field=quota | needs explicit upgrade rule")
    );
    assert!(
        stable_text
            .contains("key-identity-changed family=KeyChanged | needs explicit upgrade rule")
    );
    assert!(stable_text.contains("storage-identity-changed family=StorageMoved | unsupported"));
    assert!(
        stable_text.contains("family-hash-changed family=Account | needs explicit upgrade rule")
    );
    assert!(stable_text.contains("heuristic-limit explanation=field rename heuristic pairs removed and added fields only when their type text is identical within the same family"));
    assert!(stable_text.contains("heuristic-limit explanation=family rename heuristic requires matching storage plus key identity, or at least two same-name fields with identical types"));

    let classifications = report
        .changes()
        .iter()
        .map(|change| change.classification())
        .collect::<Vec<_>>();
    assert!(classifications.contains(&ChangeClassification::AutoSafe));
    assert!(classifications.contains(&ChangeClassification::NeedsExplicitUpgradeRule));
    assert!(classifications.contains(&ChangeClassification::Unsupported));
    assert!(report.changes().iter().any(|change| matches!(
        change.kind(),
        SchemaChangeKind::FamilyHashChanged { family } if family.as_str() == "Account"
    )));
}

fn family<const KEY_COUNT: usize, const FIELD_COUNT: usize>(
    identity: &str,
    storage: &str,
    key: [&str; KEY_COUNT],
    hash: &str,
    fields: [(&str, &str); FIELD_COUNT],
) -> FamilySchema {
    FamilySchema::new(
        FamilyIdentity::new(identity),
        StorageIdentity::new(storage),
        KeyIdentity::new(key.into_iter().map(FieldIdentity::new)),
        FamilySchemaHash::new(hash),
        fields.into_iter().map(|(field, field_type)| {
            FieldSchema::new(FieldIdentity::new(field), FieldType::new(field_type))
        }),
    )
}
