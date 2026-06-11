use upgrade::schema::lib::{
    Attempt, Block, BlockReason, Completion, ComponentName, ContractVersion, ForceFlip,
    ForceReason, Input, InputRoute, NexusAction, NexusActionRoute, NexusWork, ObjectName,
    OriginRoute, Output, OutputRoute, RawByte, RawBytes, SelectorVersion, SemaWriteInput,
    SemaWriteInputRoute, SemaWriteOutput, SignalObjectName, TraceEvent, Version, VersionLabel,
};

fn version(major: u64, minor: u64, patch: u64) -> Version {
    Version {
        major,
        minor,
        patch,
    }
}

fn contract_version(byte: u64) -> ContractVersion {
    ContractVersion::new(RawBytes::new(vec![RawByte::new(byte); 32]))
}

fn selector_version(label: &str, byte: u64) -> SelectorVersion {
    SelectorVersion {
        label: VersionLabel::new(label),
        contract_version: contract_version(byte),
    }
}

fn attempt() -> Attempt {
    Attempt {
        component: ComponentName::new("persona-spirit"),
        source: version(0, 1, 0),
        target: version(0, 1, 1),
    }
}

fn completion() -> Completion {
    Completion {
        component: ComponentName::new("persona-spirit"),
        source: version(0, 1, 0),
        target: version(0, 1, 1),
        migration: "persona-spirit-0-1-0-to-0-1-1".to_owned().into(),
        changed_records: 3,
    }
}

fn force_flip() -> ForceFlip {
    ForceFlip {
        component: ComponentName::new("persona-spirit"),
        current_version: selector_version("v0.1.0", 1),
        target_version: selector_version("v0.1.1", 2),
        reason: ForceReason::OperatorOverride,
    }
}

#[test]
fn generated_runtime_input_owns_short_header_and_frame() {
    let input = Input::attempt_upgrade(attempt());

    assert_eq!(input.route(), InputRoute::AttemptUpgrade);

    let frame = input.encode_signal_frame().expect("encode generated input");
    let (route, decoded) = Input::decode_signal_frame(&frame).expect("decode generated input");

    assert_eq!(route, InputRoute::AttemptUpgrade);
    assert_eq!(decoded, input);
}

#[test]
fn generated_runtime_signal_nexus_sema_projection_routes_attempt_upgrade() {
    let action = NexusWork::signal_arrived(Input::attempt_upgrade(attempt()))
        .with_origin_route(OriginRoute::new(17))
        .into_nexus_action();

    assert_eq!(action.origin_route(), OriginRoute::new(17));
    assert_eq!(action.root().route(), NexusActionRoute::CommandSemaWrite);
    match action.root() {
        NexusAction::CommandSemaWrite(SemaWriteInput::AttemptUpgrade(payload)) => {
            assert_eq!(payload.component.payload(), "persona-spirit");
        }
        other => panic!("expected AttemptUpgrade SEMA write, got {other:?}"),
    }

    let sema_input = action.into_sema_write_input();
    assert_eq!(sema_input.origin_route(), OriginRoute::new(17));
    assert_eq!(
        sema_input.root().route(),
        SemaWriteInputRoute::AttemptUpgrade
    );
}

#[test]
fn generated_runtime_owner_operation_projects_to_sema_write() {
    let action = NexusWork::signal_arrived(Input::force_flip(force_flip()))
        .with_origin_route(OriginRoute::new(19))
        .into_nexus_action();

    assert_eq!(action.origin_route(), OriginRoute::new(19));
    match action.root() {
        NexusAction::CommandSemaWrite(SemaWriteInput::ForceFlip(payload)) => {
            assert_eq!(payload.reason, ForceReason::OperatorOverride);
        }
        other => panic!("expected ForceFlip SEMA write, got {other:?}"),
    }
}

#[test]
fn generated_runtime_sema_completion_projects_back_to_signal_output() {
    let output = SemaWriteOutput::upgrade_completed(completion())
        .with_origin_route(OriginRoute::new(29))
        .into_nexus_work()
        .into_nexus_action()
        .into_signal_output();

    assert_eq!(output.origin_route(), OriginRoute::new(29));
    assert_eq!(output.root().route(), OutputRoute::UpgradeCompleted);
    match output.into_root() {
        Output::UpgradeCompleted(completion) => {
            assert_eq!(completion.changed_records, 3);
        }
        other => panic!("expected UpgradeCompleted output, got {other:?}"),
    }
}

#[test]
fn generated_runtime_owner_reply_projects_back_to_signal_output() {
    let output = SemaWriteOutput::blocked(Block {
        component: ComponentName::new("persona-spirit"),
        source: version(0, 1, 0),
        target: version(0, 1, 2),
        reason: BlockReason::Unsafe,
    })
    .with_origin_route(OriginRoute::new(31))
    .into_nexus_work()
    .into_nexus_action()
    .into_signal_output();

    assert_eq!(output.origin_route(), OriginRoute::new(31));
    assert_eq!(output.root().route(), OutputRoute::Blocked);
}

#[test]
fn generated_runtime_trace_vocabulary_names_signal_operation() {
    let trace = TraceEvent::new(ObjectName::Signal(SignalObjectName::Input(
        InputRoute::AttemptUpgrade,
    )));

    assert_eq!(trace.name(), "SignalInputAttemptUpgrade");
}
