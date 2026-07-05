use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemaSchemaSnapshot {
    families: BTreeMap<FamilyIdentity, FamilySchema>,
}

impl SemaSchemaSnapshot {
    pub fn new(families: impl IntoIterator<Item = FamilySchema>) -> Self {
        Self {
            families: families
                .into_iter()
                .map(|family| (family.identity.clone(), family))
                .collect(),
        }
    }

    pub fn difference_from(&self, old: &Self) -> SchemaDifferenceReport {
        SchemaDifference::new(old, self).report()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FamilyIdentity(String);

impl FamilyIdentity {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilySchema {
    identity: FamilyIdentity,
    storage_identity: StorageIdentity,
    key: KeyIdentity,
    schema_hash: FamilySchemaHash,
    fields: BTreeMap<FieldIdentity, FieldSchema>,
}

impl FamilySchema {
    pub fn new(
        identity: FamilyIdentity,
        storage_identity: StorageIdentity,
        key: KeyIdentity,
        schema_hash: FamilySchemaHash,
        fields: impl IntoIterator<Item = FieldSchema>,
    ) -> Self {
        Self {
            identity,
            storage_identity,
            key,
            schema_hash,
            fields: fields
                .into_iter()
                .map(|field| (field.identity.clone(), field))
                .collect(),
        }
    }

    pub fn identity(&self) -> &FamilyIdentity {
        &self.identity
    }

    fn shape_similarity_to(&self, other: &Self) -> ShapeSimilarity {
        let shared_storage = self.storage_identity == other.storage_identity;
        let shared_key = self.key == other.key;
        let shared_field_count = self
            .fields
            .iter()
            .filter(|(identity, field)| {
                other
                    .fields
                    .get(identity)
                    .is_some_and(|other_field| field.field_type == other_field.field_type)
            })
            .count();
        ShapeSimilarity::new(shared_storage, shared_key, shared_field_count)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct StorageIdentity(String);

impl StorageIdentity {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct KeyIdentity(Vec<FieldIdentity>);

impl KeyIdentity {
    pub fn new(fields: impl IntoIterator<Item = FieldIdentity>) -> Self {
        Self(fields.into_iter().collect())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FamilySchemaHash(String);

impl FamilySchemaHash {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FieldIdentity(String);

impl FieldIdentity {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldSchema {
    identity: FieldIdentity,
    field_type: FieldType,
}

impl FieldSchema {
    pub fn new(identity: FieldIdentity, field_type: FieldType) -> Self {
        Self {
            identity,
            field_type,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FieldType(String);

impl FieldType {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaDifferenceReport {
    changes: Vec<SchemaChange>,
}

impl SchemaDifferenceReport {
    pub fn changes(&self) -> &[SchemaChange] {
        &self.changes
    }

    pub fn stable_text(&self) -> String {
        self.changes
            .iter()
            .map(SchemaChange::stable_text)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaChange {
    kind: SchemaChangeKind,
    classification: ChangeClassification,
    facts: Vec<SchemaFact>,
}

impl SchemaChange {
    pub fn kind(&self) -> &SchemaChangeKind {
        &self.kind
    }

    pub fn classification(&self) -> ChangeClassification {
        self.classification
    }

    pub fn facts(&self) -> &[SchemaFact] {
        &self.facts
    }

    fn new(
        kind: SchemaChangeKind,
        classification: ChangeClassification,
        facts: impl IntoIterator<Item = SchemaFact>,
    ) -> Self {
        Self {
            kind,
            classification,
            facts: facts.into_iter().collect(),
        }
    }

    fn stable_text(&self) -> String {
        let facts = self
            .facts
            .iter()
            .map(SchemaFact::stable_text)
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{} | {} | {}",
            self.kind.stable_text(),
            self.classification.stable_text(),
            facts
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaChangeKind {
    AddedFamily {
        family: FamilyIdentity,
    },
    RemovedFamily {
        family: FamilyIdentity,
    },
    LikelyRenamedFamily {
        old_family: FamilyIdentity,
        new_family: FamilyIdentity,
    },
    AddedField {
        family: FamilyIdentity,
        field: FieldIdentity,
    },
    RemovedField {
        family: FamilyIdentity,
        field: FieldIdentity,
    },
    LikelyRenamedField {
        family: FamilyIdentity,
        old_field: FieldIdentity,
        new_field: FieldIdentity,
    },
    TypeChanged {
        family: FamilyIdentity,
        field: FieldIdentity,
    },
    KeyIdentityChanged {
        family: FamilyIdentity,
    },
    StorageIdentityChanged {
        family: FamilyIdentity,
    },
    FamilyHashChanged {
        family: FamilyIdentity,
    },
}

impl SchemaChangeKind {
    fn stable_text(&self) -> String {
        match self {
            Self::AddedFamily { family } => format!("added-family family={}", family.as_str()),
            Self::RemovedFamily { family } => format!("removed-family family={}", family.as_str()),
            Self::LikelyRenamedFamily {
                old_family,
                new_family,
            } => format!(
                "likely-renamed-family old={} new={}",
                old_family.as_str(),
                new_family.as_str()
            ),
            Self::AddedField { family, field } => {
                format!(
                    "added-field family={} field={}",
                    family.as_str(),
                    field.as_str()
                )
            }
            Self::RemovedField { family, field } => format!(
                "removed-field family={} field={}",
                family.as_str(),
                field.as_str()
            ),
            Self::LikelyRenamedField {
                family,
                old_field,
                new_field,
            } => format!(
                "likely-renamed-field family={} old={} new={}",
                family.as_str(),
                old_field.as_str(),
                new_field.as_str()
            ),
            Self::TypeChanged { family, field } => format!(
                "type-changed family={} field={}",
                family.as_str(),
                field.as_str()
            ),
            Self::KeyIdentityChanged { family } => {
                format!("key-identity-changed family={}", family.as_str())
            }
            Self::StorageIdentityChanged { family } => {
                format!("storage-identity-changed family={}", family.as_str())
            }
            Self::FamilyHashChanged { family } => {
                format!("family-hash-changed family={}", family.as_str())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeClassification {
    AutoSafe,
    NeedsExplicitUpgradeRule,
    Unsupported,
}

impl ChangeClassification {
    fn stable_text(self) -> &'static str {
        match self {
            Self::AutoSafe => "auto-safe",
            Self::NeedsExplicitUpgradeRule => "needs explicit upgrade rule",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaFact {
    FamilyPresentOnlyInNew,
    FamilyPresentOnlyInOld,
    FamilyNamesDiffer,
    FieldPresentOnlyInNew,
    FieldPresentOnlyInOld,
    FieldNamesDiffer,
    FieldTypeChanged {
        old_type: FieldType,
        new_type: FieldType,
    },
    FieldTypesMatch {
        field_type: FieldType,
    },
    StorageIdentityMatches,
    StorageIdentityChanged {
        old_storage: StorageIdentity,
        new_storage: StorageIdentity,
    },
    KeyIdentityMatches,
    KeyIdentityChanged {
        old_key: KeyIdentity,
        new_key: KeyIdentity,
    },
    FamilyHashChanged {
        old_hash: FamilySchemaHash,
        new_hash: FamilySchemaHash,
    },
    SharedTypedFieldCount {
        count: usize,
    },
    HeuristicLimit {
        explanation: &'static str,
    },
}

impl SchemaFact {
    fn stable_text(&self) -> String {
        match self {
            Self::FamilyPresentOnlyInNew => "family-present-only-in-new".to_string(),
            Self::FamilyPresentOnlyInOld => "family-present-only-in-old".to_string(),
            Self::FamilyNamesDiffer => "family-names-differ".to_string(),
            Self::FieldPresentOnlyInNew => "field-present-only-in-new".to_string(),
            Self::FieldPresentOnlyInOld => "field-present-only-in-old".to_string(),
            Self::FieldNamesDiffer => "field-names-differ".to_string(),
            Self::FieldTypeChanged { old_type, new_type } => {
                format!("field-type-changed old={} new={}", old_type.0, new_type.0)
            }
            Self::FieldTypesMatch { field_type } => {
                format!("field-types-match type={}", field_type.0)
            }
            Self::StorageIdentityMatches => "storage-identity-matches".to_string(),
            Self::StorageIdentityChanged {
                old_storage,
                new_storage,
            } => format!(
                "storage-identity-changed old={} new={}",
                old_storage.0, new_storage.0
            ),
            Self::KeyIdentityMatches => "key-identity-matches".to_string(),
            Self::KeyIdentityChanged { old_key, new_key } => format!(
                "key-identity-changed old={} new={}",
                old_key.stable_text(),
                new_key.stable_text()
            ),
            Self::FamilyHashChanged { old_hash, new_hash } => {
                format!("family-hash-changed old={} new={}", old_hash.0, new_hash.0)
            }
            Self::SharedTypedFieldCount { count } => {
                format!("shared-typed-field-count count={count}")
            }
            Self::HeuristicLimit { explanation } => {
                format!("heuristic-limit explanation={explanation}")
            }
        }
    }
}

impl KeyIdentity {
    fn stable_text(&self) -> String {
        self.0
            .iter()
            .map(FieldIdentity::as_str)
            .collect::<Vec<_>>()
            .join("+")
    }
}

struct SchemaDifference<'a> {
    old: &'a SemaSchemaSnapshot,
    new: &'a SemaSchemaSnapshot,
}

impl<'a> SchemaDifference<'a> {
    fn new(old: &'a SemaSchemaSnapshot, new: &'a SemaSchemaSnapshot) -> Self {
        Self { old, new }
    }

    fn report(&self) -> SchemaDifferenceReport {
        let family_renames = self.family_renames();
        let mut changes = Vec::new();
        changes.extend(self.family_presence_changes(&family_renames));
        changes.extend(self.matched_family_changes(&family_renames));
        changes.sort_by_key(SchemaChange::stable_text);
        SchemaDifferenceReport { changes }
    }

    fn family_renames(&self) -> BTreeMap<FamilyIdentity, FamilyIdentity> {
        let removed = self.removed_families();
        let added = self.added_families();
        let mut rename_by_removed = BTreeMap::new();
        let mut claimed_new = BTreeSet::new();

        for old_identity in removed {
            let old_family = &self.old.families[&old_identity];
            let best = added
                .iter()
                .filter(|new_identity| !claimed_new.contains(*new_identity))
                .filter_map(|new_identity| {
                    let new_family = &self.new.families[new_identity];
                    old_family
                        .shape_similarity_to(new_family)
                        .qualifies_as_likely_family_rename()
                        .then_some((
                            new_identity.clone(),
                            old_family.shape_similarity_to(new_family),
                        ))
                })
                .max_by_key(|(new_identity, similarity)| {
                    (similarity.score(), new_identity.clone())
                });

            if let Some((new_identity, _)) = best {
                claimed_new.insert(new_identity.clone());
                rename_by_removed.insert(old_identity, new_identity);
            }
        }

        rename_by_removed
    }

    fn family_presence_changes(
        &self,
        family_renames: &BTreeMap<FamilyIdentity, FamilyIdentity>,
    ) -> Vec<SchemaChange> {
        let renamed_old = family_renames.keys().cloned().collect::<BTreeSet<_>>();
        let renamed_new = family_renames.values().cloned().collect::<BTreeSet<_>>();
        let mut changes = Vec::new();

        for (old_family, new_family) in family_renames {
            let old_schema = &self.old.families[old_family];
            let new_schema = &self.new.families[new_family];
            let similarity = old_schema.shape_similarity_to(new_schema);
            changes.push(SchemaChange::new(
                SchemaChangeKind::LikelyRenamedFamily {
                    old_family: old_family.clone(),
                    new_family: new_family.clone(),
                },
                ChangeClassification::NeedsExplicitUpgradeRule,
                [
                    SchemaFact::FamilyNamesDiffer,
                    similarity.storage_fact(old_schema, new_schema),
                    similarity.key_fact(old_schema, new_schema),
                    SchemaFact::SharedTypedFieldCount {
                        count: similarity.shared_field_count,
                    },
                    SchemaFact::HeuristicLimit {
                        explanation: "family rename heuristic requires matching storage plus key identity, or at least two same-name fields with identical types",
                    },
                ],
            ));
        }

        for family in self.removed_families() {
            if !renamed_old.contains(&family) {
                changes.push(SchemaChange::new(
                    SchemaChangeKind::RemovedFamily {
                        family: family.clone(),
                    },
                    ChangeClassification::Unsupported,
                    [SchemaFact::FamilyPresentOnlyInOld],
                ));
            }
        }

        for family in self.added_families() {
            if !renamed_new.contains(&family) {
                changes.push(SchemaChange::new(
                    SchemaChangeKind::AddedFamily {
                        family: family.clone(),
                    },
                    ChangeClassification::AutoSafe,
                    [SchemaFact::FamilyPresentOnlyInNew],
                ));
            }
        }

        changes
    }

    fn matched_family_changes(
        &self,
        family_renames: &BTreeMap<FamilyIdentity, FamilyIdentity>,
    ) -> Vec<SchemaChange> {
        let mut changes = Vec::new();
        let mut pairs = self
            .old
            .families
            .keys()
            .filter_map(|family| {
                self.new
                    .families
                    .contains_key(family)
                    .then_some((family.clone(), family.clone()))
            })
            .collect::<Vec<_>>();
        pairs.extend(
            family_renames
                .iter()
                .map(|(old_family, new_family)| (old_family.clone(), new_family.clone())),
        );

        for (old_identity, new_identity) in pairs {
            let old_family = &self.old.families[&old_identity];
            let new_family = &self.new.families[&new_identity];
            let report_family = new_family.identity.clone();
            changes.extend(self.family_identity_changes(&report_family, old_family, new_family));
            changes.extend(self.field_changes(&report_family, old_family, new_family));
        }

        changes
    }

    fn family_identity_changes(
        &self,
        report_family: &FamilyIdentity,
        old_family: &FamilySchema,
        new_family: &FamilySchema,
    ) -> Vec<SchemaChange> {
        let mut changes = Vec::new();
        if old_family.storage_identity != new_family.storage_identity {
            changes.push(SchemaChange::new(
                SchemaChangeKind::StorageIdentityChanged {
                    family: report_family.clone(),
                },
                ChangeClassification::Unsupported,
                [SchemaFact::StorageIdentityChanged {
                    old_storage: old_family.storage_identity.clone(),
                    new_storage: new_family.storage_identity.clone(),
                }],
            ));
        }
        if old_family.key != new_family.key {
            changes.push(SchemaChange::new(
                SchemaChangeKind::KeyIdentityChanged {
                    family: report_family.clone(),
                },
                ChangeClassification::NeedsExplicitUpgradeRule,
                [SchemaFact::KeyIdentityChanged {
                    old_key: old_family.key.clone(),
                    new_key: new_family.key.clone(),
                }],
            ));
        }
        if old_family.schema_hash != new_family.schema_hash {
            changes.push(SchemaChange::new(
                SchemaChangeKind::FamilyHashChanged {
                    family: report_family.clone(),
                },
                ChangeClassification::NeedsExplicitUpgradeRule,
                [SchemaFact::FamilyHashChanged {
                    old_hash: old_family.schema_hash.clone(),
                    new_hash: new_family.schema_hash.clone(),
                }],
            ));
        }
        changes
    }

    fn field_changes(
        &self,
        report_family: &FamilyIdentity,
        old_family: &FamilySchema,
        new_family: &FamilySchema,
    ) -> Vec<SchemaChange> {
        let field_renames = FieldDifference::new(old_family, new_family).field_renames();
        let renamed_old = field_renames.keys().cloned().collect::<BTreeSet<_>>();
        let renamed_new = field_renames.values().cloned().collect::<BTreeSet<_>>();
        let mut changes = Vec::new();

        for field in old_family.fields.keys() {
            if let Some(new_field) = new_family.fields.get(field) {
                let old_field = &old_family.fields[field];
                if old_field.field_type != new_field.field_type {
                    changes.push(SchemaChange::new(
                        SchemaChangeKind::TypeChanged {
                            family: report_family.clone(),
                            field: field.clone(),
                        },
                        ChangeClassification::NeedsExplicitUpgradeRule,
                        [SchemaFact::FieldTypeChanged {
                            old_type: old_field.field_type.clone(),
                            new_type: new_field.field_type.clone(),
                        }],
                    ));
                }
            }
        }

        for (old_field, new_field) in &field_renames {
            let old_schema = &old_family.fields[old_field];
            changes.push(SchemaChange::new(
                SchemaChangeKind::LikelyRenamedField {
                    family: report_family.clone(),
                    old_field: old_field.clone(),
                    new_field: new_field.clone(),
                },
                ChangeClassification::NeedsExplicitUpgradeRule,
                [
                    SchemaFact::FieldNamesDiffer,
                    SchemaFact::FieldTypesMatch {
                        field_type: old_schema.field_type.clone(),
                    },
                    SchemaFact::HeuristicLimit {
                        explanation: "field rename heuristic pairs removed and added fields only when their type text is identical within the same family",
                    },
                ],
            ));
        }

        for field in old_family.fields.keys() {
            if !new_family.fields.contains_key(field) && !renamed_old.contains(field) {
                changes.push(SchemaChange::new(
                    SchemaChangeKind::RemovedField {
                        family: report_family.clone(),
                        field: field.clone(),
                    },
                    ChangeClassification::NeedsExplicitUpgradeRule,
                    [SchemaFact::FieldPresentOnlyInOld],
                ));
            }
        }

        for field in new_family.fields.keys() {
            if !old_family.fields.contains_key(field) && !renamed_new.contains(field) {
                changes.push(SchemaChange::new(
                    SchemaChangeKind::AddedField {
                        family: report_family.clone(),
                        field: field.clone(),
                    },
                    ChangeClassification::AutoSafe,
                    [SchemaFact::FieldPresentOnlyInNew],
                ));
            }
        }

        changes
    }

    fn removed_families(&self) -> Vec<FamilyIdentity> {
        self.old
            .families
            .keys()
            .filter(|family| !self.new.families.contains_key(*family))
            .cloned()
            .collect()
    }

    fn added_families(&self) -> Vec<FamilyIdentity> {
        self.new
            .families
            .keys()
            .filter(|family| !self.old.families.contains_key(*family))
            .cloned()
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ShapeSimilarity {
    shared_storage: bool,
    shared_key: bool,
    shared_field_count: usize,
}

impl ShapeSimilarity {
    fn new(shared_storage: bool, shared_key: bool, shared_field_count: usize) -> Self {
        Self {
            shared_storage,
            shared_key,
            shared_field_count,
        }
    }

    fn qualifies_as_likely_family_rename(self) -> bool {
        (self.shared_storage && self.shared_key) || self.shared_field_count >= 2
    }

    fn score(self) -> usize {
        usize::from(self.shared_storage) * 4
            + usize::from(self.shared_key) * 4
            + self.shared_field_count
    }

    fn storage_fact(self, old_family: &FamilySchema, new_family: &FamilySchema) -> SchemaFact {
        if self.shared_storage {
            SchemaFact::StorageIdentityMatches
        } else {
            SchemaFact::StorageIdentityChanged {
                old_storage: old_family.storage_identity.clone(),
                new_storage: new_family.storage_identity.clone(),
            }
        }
    }

    fn key_fact(self, old_family: &FamilySchema, new_family: &FamilySchema) -> SchemaFact {
        if self.shared_key {
            SchemaFact::KeyIdentityMatches
        } else {
            SchemaFact::KeyIdentityChanged {
                old_key: old_family.key.clone(),
                new_key: new_family.key.clone(),
            }
        }
    }
}

struct FieldDifference<'a> {
    old: &'a FamilySchema,
    new: &'a FamilySchema,
}

impl<'a> FieldDifference<'a> {
    fn new(old: &'a FamilySchema, new: &'a FamilySchema) -> Self {
        Self { old, new }
    }

    fn field_renames(&self) -> BTreeMap<FieldIdentity, FieldIdentity> {
        let removed = self
            .old
            .fields
            .keys()
            .filter(|field| !self.new.fields.contains_key(*field))
            .cloned()
            .collect::<Vec<_>>();
        let added = self
            .new
            .fields
            .keys()
            .filter(|field| !self.old.fields.contains_key(*field))
            .cloned()
            .collect::<Vec<_>>();
        let mut rename_by_removed = BTreeMap::new();
        let mut claimed_new = BTreeSet::new();

        for old_field in removed {
            let old_schema = &self.old.fields[&old_field];
            if let Some(new_field) = added
                .iter()
                .filter(|new_field| !claimed_new.contains(*new_field))
                .find(|new_field| self.new.fields[*new_field].field_type == old_schema.field_type)
            {
                claimed_new.insert(new_field.clone());
                rename_by_removed.insert(old_field, new_field.clone());
            }
        }

        rename_by_removed
    }
}
