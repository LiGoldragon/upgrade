use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use sema_engine::{
    Engine, EngineOpen, EngineRecord, FamilyName, QueryPlan, RecordKey, SchemaHash, SchemaVersion,
    TableDescriptor, TableName,
};
use signal_spirit::{Date, Entry, RecordIdentifier, Time};
use signal_upgrade::{Attempt, ComponentName, Version};
use upgrade::{DatabaseMigration, MigrationCatalogue};

const SPIRIT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1);
const RECORDS: TableName = TableName::new("records");
const CURRENT_RECORDS_FAMILY: &str = "PersonaSpiritCurrentRecordsFamily";
const CURRENT_RECORDS_SCHEMA_LABEL: &str = "persona-spirit-records-v0.1.1";

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let source = source_argument()?;
    if !source.exists() {
        return Err(format!(
            "source database does not exist: {}",
            source.display()
        ));
    }

    let sandbox = sandbox_directory()?;
    let source_copy = sandbox.join("source-copy.sema");
    let target = sandbox.join("target-v0.1.1.sema");
    fs::copy(&source, &source_copy).map_err(|error| {
        format!(
            "failed to copy source database {} into sandbox: {error}",
            source.display()
        )
    })?;

    let attempt = Attempt {
        component: ComponentName::new("persona-spirit"),
        source: Version {
            major: 0,
            minor: 1,
            patch: 0,
        },
        target: Version {
            major: 0,
            minor: 1,
            patch: 1,
        },
    };
    let request = DatabaseMigration::new(attempt, &source_copy, &target);
    let completion = MigrationCatalogue::prototype()
        .migrate_database(&request)
        .map_err(|error| format!("migration failed: {error}"))?;
    let readable_records = count_current_records(&target)?;

    println!(
        "(SandboxUpgradeSucceeded {} {} [{}] [{}])",
        completion.changed_records,
        readable_records,
        source_copy.display(),
        target.display()
    );
    Ok(())
}

fn source_argument() -> Result<PathBuf, String> {
    let mut arguments = env::args_os();
    let program = arguments.next().unwrap_or_default();
    let source = arguments.next().ok_or_else(|| {
        format!(
            "usage: {} <path-to-v0.1.0-persona-spirit.sema>",
            Path::new(&program).display()
        )
    })?;
    if arguments.next().is_some() {
        return Err("expected exactly one database path argument".to_owned());
    }
    Ok(PathBuf::from(source))
}

fn sandbox_directory() -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system time error: {error}"))?
        .as_millis();
    let directory = env::temp_dir().join(format!(
        "upgrade-spirit-sandbox-{}-{timestamp}",
        process::id()
    ));
    fs::create_dir_all(&directory)
        .map_err(|error| format!("failed to create sandbox {}: {error}", directory.display()))?;
    Ok(directory)
}

fn count_current_records(path: &Path) -> Result<usize, String> {
    let mut engine = Engine::open(EngineOpen::new(path, SPIRIT_SCHEMA_VERSION))
        .map_err(|error| format!("failed to open migrated database: {error}"))?;
    let table = engine
        .register_table(CurrentRecordTable::new().descriptor())
        .map_err(|error| format!("failed to register migrated records table: {error}"))?;
    let records = engine
        .match_records(QueryPlan::all(table))
        .map_err(|error| format!("failed to read migrated records: {error}"))?;
    Ok(records.records().len())
}

struct CurrentRecordTable {
    records: TableName,
}

impl CurrentRecordTable {
    const fn new() -> Self {
        Self { records: RECORDS }
    }

    fn descriptor(&self) -> TableDescriptor<CurrentStoredRecord> {
        TableDescriptor::new(
            self.records,
            FamilyName::new(CURRENT_RECORDS_FAMILY),
            SchemaHash::for_label(CURRENT_RECORDS_SCHEMA_LABEL),
        )
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
struct CurrentStampedEntry {
    entry: Entry,
    date: Date,
    time: Time,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
struct CurrentStoredRecord {
    identifier: RecordIdentifier,
    entry: CurrentStampedEntry,
}

impl EngineRecord for CurrentStoredRecord {
    fn record_key(&self) -> RecordKey {
        RecordKey::new(self.identifier.value().to_string())
    }
}
