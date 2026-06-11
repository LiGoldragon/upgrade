use signal_frame::{AcceptedOutcome, RequestBuilder, RequestPayload, SubReply};
use signal_upgrade::{
    Attempt, ComponentName, Input, Output, RejectionReason, ReportQuery, Version,
};
use upgrade::schema::lib::{NexusEngine, SemaEngine};
use upgrade::{Engine, MigrationCatalogue};

fn attempt(source: Version, target: Version) -> Attempt {
    Attempt {
        component: ComponentName::new("persona-spirit"),
        source,
        target,
    }
}

fn supported_attempt() -> Attempt {
    attempt(version(0, 1, 0), version(0, 1, 1))
}

fn unsupported_attempt() -> Attempt {
    attempt(version(0, 1, 0), version(0, 1, 2))
}

fn version(major: u64, minor: u64, patch: u64) -> Version {
    Version {
        major,
        minor,
        patch,
    }
}

#[test]
fn module_index_names_persona_spirit_version_upgrade() {
    let index = MigrationCatalogue::prototype();
    let migrations = index.supported_migrations();

    assert_eq!(migrations.len(), 1);
    assert_eq!(migrations[0].component.payload(), "persona-spirit");
    assert_eq!(migrations[0].source, version(0, 1, 0));
    assert_eq!(migrations[0].target, version(0, 1, 1));
    assert_eq!(
        migrations[0].identifier.payload(),
        "persona-spirit-0-1-0-to-0-1-1"
    );
}

#[test]
fn engine_implements_generated_nexus_and_sema_traits() {
    fn assert_nexus_engine<Runtime: NexusEngine>() {}
    fn assert_sema_engine<Runtime: SemaEngine>() {}

    assert_nexus_engine::<Engine>();
    assert_sema_engine::<Engine>();
}

#[tokio::test]
async fn supported_upgrade_runs_through_generated_nexus_runner() {
    let mut engine = Engine::prototype();
    let reply = engine
        .execute(Input::attempt_upgrade(supported_attempt()).into_request())
        .await;

    let Output::UpgradeCompleted(completion) = first_reply(reply) else {
        panic!("expected UpgradeCompleted");
    };
    assert_eq!(completion.component.payload(), "persona-spirit");
    assert_eq!(completion.source, version(0, 1, 0));
    assert_eq!(completion.target, version(0, 1, 1));
    assert_eq!(completion.changed_records, 0);
}

#[tokio::test]
async fn unsupported_upgrade_rejects_as_typed_contract_reply() {
    let mut engine = Engine::prototype();

    let reply = engine
        .execute(Input::attempt_upgrade(unsupported_attempt()).into_request())
        .await;

    let Output::UpgradeRejected(rejection) = first_reply(reply) else {
        panic!("expected UpgradeRejected");
    };
    assert_eq!(rejection.component.payload(), "persona-spirit");
    assert_eq!(rejection.target, version(0, 1, 2));
    assert_eq!(rejection.reason, RejectionReason::UnsupportedMigration);
}

#[tokio::test]
async fn multi_operation_request_is_ordered_through_generated_runner() {
    let mut engine = Engine::prototype();
    let request = RequestBuilder::new()
        .with(Input::attempt_upgrade(supported_attempt()))
        .with(Input::report(ReportQuery::All))
        .build()
        .expect("non-empty request");

    let reply = engine.execute(request).await;

    let signal_frame::Reply::Accepted {
        outcome,
        per_operation,
    } = reply
    else {
        panic!("expected accepted reply");
    };
    assert_eq!(outcome, AcceptedOutcome::Committed);

    let (first, tail) = per_operation.into_head_and_tail();
    assert!(matches!(first, SubReply::Ok(Output::UpgradeCompleted(_))));
    assert_eq!(tail.len(), 1);
    let SubReply::Ok(Output::Reported(report)) = &tail[0] else {
        panic!("expected report reply");
    };
    assert_eq!(report.completions.len(), 1);
    assert!(report.rejections.is_empty());
}

#[test]
fn runtime_source_does_not_reintroduce_retired_executor() {
    let source = std::fs::read_to_string("src/execution.rs").expect("execution source");
    let underscore_name = ["signal", "executor"].join("_");
    let hyphen_name = ["signal", "executor"].join("-");
    let command_executor_name = ["Command", "Executor"].join("");
    let lowering_trait_name = ["Lowering", "Trait"].join("");

    assert!(!source.contains(&underscore_name));
    assert!(!source.contains(&hyphen_name));
    assert!(!source.contains(&command_executor_name));
    assert!(!source.contains(&lowering_trait_name));
}

fn first_reply(reply: signal_frame::Reply<Output>) -> Output {
    match reply {
        signal_frame::Reply::Accepted { per_operation, .. } => match per_operation.into_head() {
            SubReply::Ok(payload) => payload,
            other => panic!("expected successful first reply, got {other:?}"),
        },
        other => panic!("expected accepted reply, got {other:?}"),
    }
}
