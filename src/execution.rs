use signal_frame::{NonEmpty, Reply as FrameReply, SubReply};
use signal_upgrade::{Operation, Reply, RequestUnimplemented, UnimplementedReason};

use crate::catalogue::MigrationCatalogue;
use crate::schema::lib as schema;

trait ProjectInto<Target> {
    fn project_into(self) -> Target;
}

#[derive(Debug, Clone)]
pub struct Engine {
    index: MigrationCatalogue,
    completions: Vec<signal_upgrade::Completion>,
    rejections: Vec<signal_upgrade::Rejection>,
}

impl Engine {
    pub fn prototype() -> Self {
        Self {
            index: MigrationCatalogue::prototype(),
            completions: Vec::new(),
            rejections: Vec::new(),
        }
    }

    pub fn with_index(index: MigrationCatalogue) -> Self {
        Self {
            index,
            completions: Vec::new(),
            rejections: Vec::new(),
        }
    }

    pub async fn execute(&mut self, request: signal_upgrade::Request) -> FrameReply<Reply> {
        let mut replies = Vec::new();
        let mut operation_index = 0_u64;
        for operation in request.payloads {
            replies.push(SubReply::Ok(
                self.execute_operation(operation, schema::OriginRoute(operation_index))
                    .await,
            ));
            operation_index = operation_index.saturating_add(1);
        }
        FrameReply::committed(
            NonEmpty::try_from_vec(replies).expect("signal requests are structurally non-empty"),
        )
    }

    async fn execute_operation(
        &mut self,
        operation: Operation,
        origin_route: schema::OriginRoute,
    ) -> Reply {
        let Some(input) = operation.project_into() else {
            return Self::not_built_yet_reply();
        };
        let action = schema::NexusEngine::execute(
            self,
            schema::NexusWork::signal_arrived(input).with_origin_route(origin_route),
        )
        .await;
        action.into_signal_output().into_root().project_into()
    }

    fn inspect(&self, inspection: schema::Inspection) -> schema::InspectionReported {
        match inspection {
            schema::Inspection::All => schema::InspectionReported(
                self.index
                    .supported_migrations()
                    .into_iter()
                    .map(ProjectInto::project_into)
                    .collect(),
            ),
            schema::Inspection::Component(component) => schema::InspectionReported(
                self.index
                    .supported_migrations()
                    .into_iter()
                    .filter(|migration| migration.component.as_str() == component)
                    .map(ProjectInto::project_into)
                    .collect(),
            ),
        }
    }

    fn attempt_upgrade(&mut self, attempt: schema::Attempt) -> schema::SemaWriteOutput {
        let signal_attempt = attempt.project_into();
        match self.index.attempt(&signal_attempt) {
            Ok(completion) => {
                self.completions.push(completion.clone());
                schema::SemaWriteOutput::UpgradeCompleted(completion.project_into())
            }
            Err(rejection) => {
                self.rejections.push(rejection.clone());
                schema::SemaWriteOutput::UpgradeRejected(rejection.project_into())
            }
        }
    }

    fn report(&self, query: schema::ReportQuery) -> schema::Reported {
        match query {
            schema::ReportQuery::All => schema::Reported {
                completions: self
                    .completions
                    .iter()
                    .cloned()
                    .map(ProjectInto::project_into)
                    .collect(),
                rejections: self
                    .rejections
                    .iter()
                    .cloned()
                    .map(ProjectInto::project_into)
                    .collect(),
            },
            schema::ReportQuery::Component(component) => schema::Reported {
                completions: self
                    .completions
                    .iter()
                    .filter(|completion| completion.component.as_str() == component)
                    .cloned()
                    .map(ProjectInto::project_into)
                    .collect(),
                rejections: self
                    .rejections
                    .iter()
                    .filter(|rejection| rejection.component.as_str() == component)
                    .cloned()
                    .map(ProjectInto::project_into)
                    .collect(),
            },
        }
    }

    fn not_built_yet_output() -> schema::Output {
        schema::Output::RequestUnimplemented(Self::schema_not_built_yet())
    }

    fn not_built_yet_write_output() -> schema::SemaWriteOutput {
        schema::SemaWriteOutput::RequestUnimplemented(Self::schema_not_built_yet())
    }

    fn not_built_yet_read_output() -> schema::SemaReadOutput {
        schema::SemaReadOutput::RequestUnimplemented(Self::schema_not_built_yet())
    }

    fn schema_not_built_yet() -> schema::RequestUnimplemented {
        schema::RequestUnimplemented(schema::UnimplementedReason::NotBuiltYet)
    }

    fn not_built_yet_reply() -> Reply {
        Reply::RequestUnimplemented(RequestUnimplemented {
            reason: UnimplementedReason::NotBuiltYet,
        })
    }
}

impl schema::NexusEngine for Engine {
    async fn apply_sema_write(
        &mut self,
        origin_route: schema::OriginRoute,
        input: schema::SemaWriteInput,
    ) -> schema::SemaWriteOutput {
        schema::SemaEngine::apply(self, input.with_origin_route(origin_route)).into_root()
    }

    async fn observe_sema_read(
        &mut self,
        origin_route: schema::OriginRoute,
        input: schema::SemaReadInput,
    ) -> schema::SemaReadOutput {
        schema::SemaEngine::observe(self, input.with_origin_route(origin_route)).into_root()
    }

    async fn run_effect(&mut self, input: schema::NexusEffectCommand) -> schema::NexusEffectResult {
        match input {
            schema::NexusEffectCommand::CallHandoverPeer(payload) => {
                schema::NexusEffectResult::HandoverPeerCalled(schema::MirrorAcknowledgement {
                    component: payload.component,
                    mirrored_write_count: 0,
                })
            }
            schema::NexusEffectCommand::NotifySelector(payload) => {
                schema::NexusEffectResult::SelectorNotified(schema::ForcedFlip {
                    component: payload.component,
                    active_version: payload.target_version,
                })
            }
        }
    }

    fn budget_exhausted_reply(
        &self,
        _exhausted: triad_runtime::ContinuationExhausted,
    ) -> schema::Output {
        Self::not_built_yet_output()
    }

    fn decide(
        &mut self,
        input: schema::nexus::Nexus<schema::nexus::Work>,
    ) -> schema::nexus::Nexus<schema::nexus::Action> {
        input.into_nexus_action()
    }
}

impl schema::SemaEngine for Engine {
    fn apply_inner(
        &mut self,
        input: schema::sema::Sema<schema::sema::WriteInput>,
    ) -> schema::sema::Sema<schema::sema::WriteOutput> {
        let origin_route = input.origin_route();
        let output = match input.into_root() {
            schema::SemaWriteInput::AttemptUpgrade(attempt) => self.attempt_upgrade(attempt),
            schema::SemaWriteInput::ReadyToHandover(_)
            | schema::SemaWriteInput::HandoverCompleted(_)
            | schema::SemaWriteInput::Mirror(_)
            | schema::SemaWriteInput::Divergence(_)
            | schema::SemaWriteInput::RecoverFromFailure(_)
            | schema::SemaWriteInput::Register(_)
            | schema::SemaWriteInput::Allow(_)
            | schema::SemaWriteInput::Block(_)
            | schema::SemaWriteInput::ForceFlip(_)
            | schema::SemaWriteInput::Rollback(_)
            | schema::SemaWriteInput::Quarantine(_) => Self::not_built_yet_write_output(),
        };
        output.with_origin_route(origin_route)
    }

    fn observe_inner(
        &self,
        input: schema::sema::Sema<schema::sema::ReadInput>,
    ) -> schema::sema::Sema<schema::sema::ReadOutput> {
        let origin_route = input.origin_route();
        let output = match input.into_root() {
            schema::SemaReadInput::Inspect(inspection) => {
                schema::SemaReadOutput::InspectionReported(self.inspect(inspection))
            }
            schema::SemaReadInput::Report(query) => {
                schema::SemaReadOutput::Reported(self.report(query))
            }
            schema::SemaReadInput::AskHandoverMarker(_) | schema::SemaReadInput::Query(_) => {
                Self::not_built_yet_read_output()
            }
        };
        output.with_origin_route(origin_route)
    }
}

impl ProjectInto<Option<schema::Input>> for Operation {
    fn project_into(self) -> Option<schema::Input> {
        match self {
            Operation::Inspect(payload) => Some(schema::Input::Inspect(payload.project_into())),
            Operation::AttemptUpgrade(payload) => {
                Some(schema::Input::AttemptUpgrade(payload.project_into()))
            }
            Operation::Report(payload) => Some(schema::Input::Report(payload.project_into())),
            Operation::AskHandoverMarker(_)
            | Operation::ReadyToHandover(_)
            | Operation::HandoverCompleted(_)
            | Operation::Mirror(_)
            | Operation::Divergence(_)
            | Operation::RecoverFromFailure(_)
            | Operation::Tap(_)
            | Operation::Untap(_) => None,
        }
    }
}

impl ProjectInto<schema::Inspection> for signal_upgrade::Inspection {
    fn project_into(self) -> schema::Inspection {
        match self {
            Self::All => schema::Inspection::All,
            Self::Component(component) => {
                schema::Inspection::Component(component.as_str().to_owned())
            }
        }
    }
}

impl ProjectInto<schema::Attempt> for signal_upgrade::Attempt {
    fn project_into(self) -> schema::Attempt {
        schema::Attempt {
            component: self.component.as_str().to_owned(),
            source: self.source.project_into(),
            target: self.target.project_into(),
        }
    }
}

impl ProjectInto<schema::ReportQuery> for signal_upgrade::ReportQuery {
    fn project_into(self) -> schema::ReportQuery {
        match self {
            Self::All => schema::ReportQuery::All,
            Self::Component(component) => {
                schema::ReportQuery::Component(component.as_str().to_owned())
            }
        }
    }
}

impl ProjectInto<schema::Version> for signal_upgrade::Version {
    fn project_into(self) -> schema::Version {
        schema::Version {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
        }
    }
}

impl ProjectInto<signal_upgrade::Version> for schema::Version {
    fn project_into(self) -> signal_upgrade::Version {
        signal_upgrade::Version::new(self.major, self.minor, self.patch)
    }
}

impl ProjectInto<signal_upgrade::Attempt> for schema::Attempt {
    fn project_into(self) -> signal_upgrade::Attempt {
        signal_upgrade::Attempt {
            component: signal_upgrade::ComponentName::new(self.component),
            source: self.source.project_into(),
            target: self.target.project_into(),
        }
    }
}

impl ProjectInto<schema::SupportedMigration> for signal_upgrade::SupportedMigration {
    fn project_into(self) -> schema::SupportedMigration {
        schema::SupportedMigration {
            component: self.component.as_str().to_owned(),
            source: self.source.project_into(),
            target: self.target.project_into(),
            identifier: self.identifier.as_str().to_owned(),
        }
    }
}

impl ProjectInto<schema::Completion> for signal_upgrade::Completion {
    fn project_into(self) -> schema::Completion {
        schema::Completion {
            component: self.component.as_str().to_owned(),
            source: self.source.project_into(),
            target: self.target.project_into(),
            migration: self.migration.as_str().to_owned(),
            changed_records: self.changed_records,
        }
    }
}

impl ProjectInto<signal_upgrade::Completion> for schema::Completion {
    fn project_into(self) -> signal_upgrade::Completion {
        signal_upgrade::Completion {
            component: signal_upgrade::ComponentName::new(self.component),
            source: self.source.project_into(),
            target: self.target.project_into(),
            migration: signal_upgrade::MigrationIdentifier::new(self.migration),
            changed_records: self.changed_records,
        }
    }
}

impl ProjectInto<schema::RejectionReason> for signal_upgrade::RejectionReason {
    fn project_into(self) -> schema::RejectionReason {
        match self {
            Self::UnsupportedMigration => schema::RejectionReason::UnsupportedMigration,
            Self::ComponentMismatch => schema::RejectionReason::ComponentMismatch,
            Self::MigrationFailed => schema::RejectionReason::MigrationFailed,
        }
    }
}

impl ProjectInto<signal_upgrade::RejectionReason> for schema::RejectionReason {
    fn project_into(self) -> signal_upgrade::RejectionReason {
        match self {
            Self::UnsupportedMigration => signal_upgrade::RejectionReason::UnsupportedMigration,
            Self::ComponentMismatch => signal_upgrade::RejectionReason::ComponentMismatch,
            Self::MigrationFailed => signal_upgrade::RejectionReason::MigrationFailed,
        }
    }
}

impl ProjectInto<schema::Rejection> for signal_upgrade::Rejection {
    fn project_into(self) -> schema::Rejection {
        schema::Rejection {
            component: self.component.as_str().to_owned(),
            source: self.source.project_into(),
            target: self.target.project_into(),
            reason: self.reason.project_into(),
        }
    }
}

impl ProjectInto<signal_upgrade::Rejection> for schema::Rejection {
    fn project_into(self) -> signal_upgrade::Rejection {
        signal_upgrade::Rejection {
            component: signal_upgrade::ComponentName::new(self.component),
            source: self.source.project_into(),
            target: self.target.project_into(),
            reason: self.reason.project_into(),
        }
    }
}

impl ProjectInto<signal_upgrade::InspectionReported> for schema::InspectionReported {
    fn project_into(self) -> signal_upgrade::InspectionReported {
        signal_upgrade::InspectionReported {
            migrations: self
                .0
                .into_iter()
                .map(|migration| signal_upgrade::SupportedMigration {
                    component: signal_upgrade::ComponentName::new(migration.component),
                    source: migration.source.project_into(),
                    target: migration.target.project_into(),
                    identifier: signal_upgrade::MigrationIdentifier::new(migration.identifier),
                })
                .collect(),
        }
    }
}

impl ProjectInto<signal_upgrade::Reported> for schema::Reported {
    fn project_into(self) -> signal_upgrade::Reported {
        signal_upgrade::Reported {
            completions: self
                .completions
                .into_iter()
                .map(ProjectInto::project_into)
                .collect(),
            rejections: self
                .rejections
                .into_iter()
                .map(ProjectInto::project_into)
                .collect(),
        }
    }
}

impl ProjectInto<Reply> for schema::Output {
    fn project_into(self) -> Reply {
        match self {
            schema::Output::InspectionReported(payload) => {
                Reply::InspectionReported(payload.project_into())
            }
            schema::Output::UpgradeCompleted(payload) => {
                Reply::UpgradeCompleted(payload.project_into())
            }
            schema::Output::UpgradeRejected(payload) => {
                Reply::UpgradeRejected(payload.project_into())
            }
            schema::Output::Reported(payload) => Reply::Reported(payload.project_into()),
            schema::Output::RequestUnimplemented(payload) => {
                Reply::RequestUnimplemented(payload.project_into())
            }
            schema::Output::HandoverMarker(_)
            | schema::Output::HandoverAccepted(_)
            | schema::Output::HandoverFinalized(_)
            | schema::Output::MirrorAcknowledged(_)
            | schema::Output::DivergenceAcknowledged(_)
            | schema::Output::RecoveryCompleted(_)
            | schema::Output::HandoverRejected(_)
            | schema::Output::Registered(_)
            | schema::Output::Allowed(_)
            | schema::Output::Blocked(_)
            | schema::Output::PolicyReported(_)
            | schema::Output::PolicyRejected(_)
            | schema::Output::FlipForced(_)
            | schema::Output::RolledBack(_)
            | schema::Output::Quarantined(_)
            | schema::Output::Rejected(_) => Engine::not_built_yet_reply(),
        }
    }
}

impl ProjectInto<RequestUnimplemented> for schema::RequestUnimplemented {
    fn project_into(self) -> RequestUnimplemented {
        RequestUnimplemented {
            reason: match self.0 {
                schema::UnimplementedReason::NotBuiltYet => UnimplementedReason::NotBuiltYet,
                schema::UnimplementedReason::IntegrationNotLanded => {
                    UnimplementedReason::IntegrationNotLanded
                }
            },
        }
    }
}
