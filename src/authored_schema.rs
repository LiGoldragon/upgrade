use std::{collections::BTreeMap, fs, path::Path};

use schema::{
    FamilyKey, SchemaEngine, SchemaError, SchemaIdentity, TypeDeclaration, TypeReference,
};
use thiserror::Error;

use crate::{
    FamilyIdentity, FamilySchema, FamilySchemaHash, FieldIdentity, FieldSchema, FieldType,
    KeyIdentity, SemaSchemaSnapshot, StorageIdentity,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredSchemaSnapshot {
    snapshot: SemaSchemaSnapshot,
    missing_facts: Vec<AuthoredSchemaMissingFact>,
}

impl AuthoredSchemaSnapshot {
    pub fn from_schema_file(
        path: impl AsRef<Path>,
        identity: AuthoredSchemaIdentity,
    ) -> Result<Self, AuthoredSchemaError> {
        let source = fs::read_to_string(path.as_ref()).map_err(|source| {
            AuthoredSchemaError::ReadSchemaFile {
                path: path.as_ref().display().to_string(),
                source,
            }
        })?;
        Self::from_schema_text(&source, identity)
    }

    pub fn from_schema_text(
        source: &str,
        identity: AuthoredSchemaIdentity,
    ) -> Result<Self, AuthoredSchemaError> {
        AuthoredSchemaAdapter::new(source, identity).adapt()
    }

    pub fn snapshot(&self) -> &SemaSchemaSnapshot {
        &self.snapshot
    }

    pub fn into_snapshot(self) -> SemaSchemaSnapshot {
        self.snapshot
    }

    pub fn missing_facts(&self) -> &[AuthoredSchemaMissingFact] {
        &self.missing_facts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredSchemaIdentity {
    component: String,
    version: String,
}

impl AuthoredSchemaIdentity {
    pub fn new(component: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            version: version.into(),
        }
    }

    fn to_schema_identity(&self) -> SchemaIdentity {
        SchemaIdentity::new(self.component.clone(), self.version.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AuthoredSchemaMissingFact {
    FamilyRecordFieldsUnavailable {
        family: String,
        record: String,
        declaration_kind: AuthoredRecordDeclarationKind,
    },
}

impl AuthoredSchemaMissingFact {
    pub fn stable_text(&self) -> String {
        match self {
            Self::FamilyRecordFieldsUnavailable {
                family,
                record,
                declaration_kind,
            } => format!(
                "missing family-record-fields family={family} record={record} declaration-kind={}",
                declaration_kind.stable_text()
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AuthoredRecordDeclarationKind {
    Enum,
    ImportedOrAbsent,
    Newtype,
}

impl AuthoredRecordDeclarationKind {
    fn stable_text(self) -> &'static str {
        match self {
            Self::Enum => "enum",
            Self::ImportedOrAbsent => "imported-or-absent",
            Self::Newtype => "newtype",
        }
    }
}

#[derive(Debug, Error)]
pub enum AuthoredSchemaError {
    #[error("read authored schema file {path}: {source}")]
    ReadSchemaFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parse/lower authored schema: {0}")]
    Schema(#[from] SchemaError),
}

struct AuthoredSchemaAdapter<'source> {
    source: &'source str,
    identity: AuthoredSchemaIdentity,
}

impl<'source> AuthoredSchemaAdapter<'source> {
    fn new(source: &'source str, identity: AuthoredSchemaIdentity) -> Self {
        Self { source, identity }
    }

    fn adapt(&self) -> Result<AuthoredSchemaSnapshot, AuthoredSchemaError> {
        let schema = SchemaEngine::default()
            .lower_source(self.source, self.identity.to_schema_identity())?;
        let declarations = AuthoredRecordDeclarations::from_schema(&schema);
        let mut missing_facts = Vec::new();
        let mut families = Vec::new();

        for family in schema.families() {
            let family_identity = family.name.as_str().to_owned();
            let fields =
                declarations.fields_for(&family.record, &family_identity, &mut missing_facts);
            let schema_hash = schema
                .family_closure(family.record.as_str())?
                .content_hash()?
                .to_hex();
            families.push(FamilySchema::new(
                FamilyIdentity::new(family_identity),
                StorageIdentity::new(family.table.as_str()),
                KeyIdentity::new([FieldIdentity::new(
                    AuthoredFamilyKey::new(family.key).stable_text(),
                )]),
                FamilySchemaHash::new(schema_hash),
                fields,
            ));
        }

        missing_facts.sort();
        Ok(AuthoredSchemaSnapshot {
            snapshot: SemaSchemaSnapshot::new(families),
            missing_facts,
        })
    }
}

struct AuthoredRecordDeclarations<'schema> {
    declarations: BTreeMap<&'schema str, &'schema TypeDeclaration>,
}

impl<'schema> AuthoredRecordDeclarations<'schema> {
    fn from_schema(schema: &'schema schema::Schema) -> Self {
        Self {
            declarations: schema
                .namespace()
                .iter()
                .map(|declaration| (declaration.name().as_str(), declaration.value()))
                .collect(),
        }
    }

    fn fields_for(
        &self,
        record: &schema::Name,
        family: &str,
        missing_facts: &mut Vec<AuthoredSchemaMissingFact>,
    ) -> Vec<FieldSchema> {
        match self.declarations.get(record.as_str()) {
            Some(TypeDeclaration::Struct(declaration)) => declaration
                .fields
                .iter()
                .map(|field| {
                    FieldSchema::new(
                        FieldIdentity::new(field.name.as_str()),
                        FieldType::new(AuthoredTypeReference::new(&field.reference).stable_text()),
                    )
                })
                .collect(),
            Some(TypeDeclaration::Enum(_)) => {
                missing_facts.push(AuthoredSchemaMissingFact::FamilyRecordFieldsUnavailable {
                    family: family.to_owned(),
                    record: record.as_str().to_owned(),
                    declaration_kind: AuthoredRecordDeclarationKind::Enum,
                });
                Vec::new()
            }
            Some(TypeDeclaration::Newtype(_)) => {
                missing_facts.push(AuthoredSchemaMissingFact::FamilyRecordFieldsUnavailable {
                    family: family.to_owned(),
                    record: record.as_str().to_owned(),
                    declaration_kind: AuthoredRecordDeclarationKind::Newtype,
                });
                Vec::new()
            }
            None => {
                missing_facts.push(AuthoredSchemaMissingFact::FamilyRecordFieldsUnavailable {
                    family: family.to_owned(),
                    record: record.as_str().to_owned(),
                    declaration_kind: AuthoredRecordDeclarationKind::ImportedOrAbsent,
                });
                Vec::new()
            }
        }
    }
}

struct AuthoredFamilyKey {
    key: FamilyKey,
}

impl AuthoredFamilyKey {
    fn new(key: FamilyKey) -> Self {
        Self { key }
    }

    fn stable_text(&self) -> &'static str {
        match self.key {
            FamilyKey::Domain => "Domain",
            FamilyKey::Identified => "Identified",
        }
    }
}

struct AuthoredTypeReference<'reference> {
    reference: &'reference TypeReference,
}

impl<'reference> AuthoredTypeReference<'reference> {
    fn new(reference: &'reference TypeReference) -> Self {
        Self { reference }
    }

    fn stable_text(&self) -> String {
        match self.reference {
            TypeReference::String => "String".to_owned(),
            TypeReference::Integer => "Integer".to_owned(),
            TypeReference::Boolean => "Boolean".to_owned(),
            TypeReference::Path => "Path".to_owned(),
            TypeReference::Bytes => "Bytes".to_owned(),
            TypeReference::FixedBytes(width) => format!("Bytes<{width}>"),
            TypeReference::Plain(name) => name.as_str().to_owned(),
            TypeReference::Vector(reference) => {
                format!("Vector<{}>", Self::new(reference).stable_text())
            }
            TypeReference::Map(key, value) => format!(
                "Map<{},{}>",
                Self::new(key).stable_text(),
                Self::new(value).stable_text()
            ),
            TypeReference::Optional(reference) => {
                format!("Optional<{}>", Self::new(reference).stable_text())
            }
            TypeReference::ScopeOf(reference) => {
                format!("ScopeOf<{}>", Self::new(reference).stable_text())
            }
            TypeReference::Application { head, arguments } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| Self::new(argument).stable_text())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{}<{arguments}>", head.name().as_str())
            }
        }
    }
}
