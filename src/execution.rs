use signal_frame::{NonEmpty, Reply as FrameReply, SubReply};
use signal_upgrade::{Input, Output, UnimplementedReason};

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

    pub async fn execute(&mut self, request: signal_upgrade::Request) -> FrameReply<Output> {
        let mut replies = Vec::new();
        let mut operation_index = 0_u64;
        for input in request.payloads {
            replies.push(SubReply::Ok(
                self.execute_input(input, schema::OriginRoute(operation_index))
                    .await,
            ));
            operation_index = operation_index.saturating_add(1);
        }
        FrameReply::committed(
            NonEmpty::try_from_vec(replies).expect("signal requests are structurally non-empty"),
        )
    }

    async fn execute_input(&mut self, input: Input, origin_route: schema::OriginRoute) -> Output {
        let action = schema::NexusEngine::execute(
            self,
            schema::NexusWork::signal_arrived(input.project_into()).with_origin_route(origin_route),
        )
        .await;
        action.into_signal_output().into_root().project_into()
    }

    fn inspect(&self, inspection: schema::Inspection) -> schema::InspectionReported {
        match inspection {
            schema::Inspection::All => schema::InspectionReported::new(
                self.index
                    .supported_migrations()
                    .into_iter()
                    .map(ProjectInto::project_into)
                    .collect(),
            ),
            schema::Inspection::Component(component) => schema::InspectionReported::new(
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
        schema::Output::request_unimplemented(schema::UnimplementedReason::NotBuiltYet)
    }

    fn not_built_yet_write_output() -> schema::SemaWriteOutput {
        schema::SemaWriteOutput::request_unimplemented(schema::UnimplementedReason::NotBuiltYet)
    }

    fn not_built_yet_read_output() -> schema::SemaReadOutput {
        schema::SemaReadOutput::request_unimplemented(schema::UnimplementedReason::NotBuiltYet)
    }

    fn not_built_yet_reply() -> Output {
        Output::request_unimplemented(UnimplementedReason::NotBuiltYet)
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

impl ProjectInto<schema::Input> for Input {
    fn project_into(self) -> schema::Input {
        match self {
            Self::Inspect(payload) => schema::Input::Inspect(payload.project_into()),
            Self::AttemptUpgrade(payload) => schema::Input::AttemptUpgrade(payload.project_into()),
            Self::Report(payload) => schema::Input::Report(payload.project_into()),
            Self::AskHandoverMarker(payload) => {
                schema::Input::AskHandoverMarker(payload.project_into())
            }
            Self::ReadyToHandover(payload) => {
                schema::Input::ReadyToHandover(payload.project_into())
            }
            Self::HandoverCompleted(payload) => {
                schema::Input::HandoverCompleted(payload.project_into())
            }
            Self::Mirror(payload) => schema::Input::Mirror(payload.project_into()),
            Self::Divergence(payload) => schema::Input::Divergence(payload.project_into()),
            Self::RecoverFromFailure(payload) => {
                schema::Input::RecoverFromFailure(payload.project_into())
            }
        }
    }
}

impl ProjectInto<schema::Inspection> for signal_upgrade::Inspection {
    fn project_into(self) -> schema::Inspection {
        match self {
            Self::All => schema::Inspection::All,
            Self::Component(component) => schema::Inspection::Component(component),
        }
    }
}

impl ProjectInto<schema::Attempt> for signal_upgrade::Attempt {
    fn project_into(self) -> schema::Attempt {
        schema::Attempt {
            component: self.component,
            source: self.source.project_into(),
            target: self.target.project_into(),
        }
    }
}

impl ProjectInto<schema::ReportQuery> for signal_upgrade::ReportQuery {
    fn project_into(self) -> schema::ReportQuery {
        match self {
            Self::All => schema::ReportQuery::All,
            Self::Component(component) => schema::ReportQuery::Component(component),
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
        signal_upgrade::Version {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
        }
    }
}

impl ProjectInto<signal_upgrade::Attempt> for schema::Attempt {
    fn project_into(self) -> signal_upgrade::Attempt {
        signal_upgrade::Attempt {
            component: self.component,
            source: self.source.project_into(),
            target: self.target.project_into(),
        }
    }
}

impl ProjectInto<schema::SupportedMigration> for signal_upgrade::SupportedMigration {
    fn project_into(self) -> schema::SupportedMigration {
        schema::SupportedMigration {
            component: self.component,
            source: self.source.project_into(),
            target: self.target.project_into(),
            identifier: self.identifier,
        }
    }
}

impl ProjectInto<schema::Completion> for signal_upgrade::Completion {
    fn project_into(self) -> schema::Completion {
        schema::Completion {
            component: self.component,
            source: self.source.project_into(),
            target: self.target.project_into(),
            migration: self.migration,
            changed_records: self.changed_records,
        }
    }
}

impl ProjectInto<signal_upgrade::Completion> for schema::Completion {
    fn project_into(self) -> signal_upgrade::Completion {
        signal_upgrade::Completion {
            component: self.component,
            source: self.source.project_into(),
            target: self.target.project_into(),
            migration: self.migration,
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
            component: self.component,
            source: self.source.project_into(),
            target: self.target.project_into(),
            reason: self.reason.project_into(),
        }
    }
}

impl ProjectInto<signal_upgrade::Rejection> for schema::Rejection {
    fn project_into(self) -> signal_upgrade::Rejection {
        signal_upgrade::Rejection {
            component: self.component,
            source: self.source.project_into(),
            target: self.target.project_into(),
            reason: self.reason.project_into(),
        }
    }
}

impl ProjectInto<schema::ContractVersion> for signal_upgrade::ContractVersion {
    fn project_into(self) -> schema::ContractVersion {
        schema::ContractVersion::new(self.into_payload())
    }
}

impl ProjectInto<signal_upgrade::ContractVersion> for schema::ContractVersion {
    fn project_into(self) -> signal_upgrade::ContractVersion {
        signal_upgrade::ContractVersion::new(self.into_payload())
    }
}

impl ProjectInto<schema::Date> for signal_upgrade::Date {
    fn project_into(self) -> schema::Date {
        schema::Date {
            year: self.year,
            month: self.month,
            day: self.day,
        }
    }
}

impl ProjectInto<signal_upgrade::Date> for schema::Date {
    fn project_into(self) -> signal_upgrade::Date {
        signal_upgrade::Date {
            year: self.year,
            month: self.month,
            day: self.day,
        }
    }
}

impl ProjectInto<schema::Time> for signal_upgrade::Time {
    fn project_into(self) -> schema::Time {
        schema::Time {
            hour: self.hour,
            minute: self.minute,
            second: self.second,
        }
    }
}

impl ProjectInto<signal_upgrade::Time> for schema::Time {
    fn project_into(self) -> signal_upgrade::Time {
        signal_upgrade::Time {
            hour: self.hour,
            minute: self.minute,
            second: self.second,
        }
    }
}

impl ProjectInto<schema::HandoverMarker> for signal_upgrade::HandoverMarker {
    fn project_into(self) -> schema::HandoverMarker {
        schema::HandoverMarker {
            component: self.component,
            schema_hash: self.schema_hash.project_into(),
            state_sequence: self.state_sequence,
            mirrored_write_count: self.mirrored_write_count,
            record_frontier: self.record_frontier,
            recorded_at_date: self.recorded_at_date.project_into(),
            recorded_at_time: self.recorded_at_time.project_into(),
        }
    }
}

impl ProjectInto<signal_upgrade::HandoverMarker> for schema::HandoverMarker {
    fn project_into(self) -> signal_upgrade::HandoverMarker {
        signal_upgrade::HandoverMarker {
            component: self.component,
            schema_hash: self.schema_hash.project_into(),
            state_sequence: self.state_sequence,
            mirrored_write_count: self.mirrored_write_count,
            record_frontier: self.record_frontier,
            recorded_at_date: self.recorded_at_date.project_into(),
            recorded_at_time: self.recorded_at_time.project_into(),
        }
    }
}

impl ProjectInto<schema::MarkerRequest> for signal_upgrade::MarkerRequest {
    fn project_into(self) -> schema::MarkerRequest {
        schema::MarkerRequest::new(self.into_payload())
    }
}

impl ProjectInto<schema::ReadinessReport> for signal_upgrade::ReadinessReport {
    fn project_into(self) -> schema::ReadinessReport {
        schema::ReadinessReport {
            component: self.component,
            source_marker: self.source_marker.project_into(),
        }
    }
}

impl ProjectInto<schema::CompletionReport> for signal_upgrade::CompletionReport {
    fn project_into(self) -> schema::CompletionReport {
        schema::CompletionReport {
            component: self.component,
            accepted_marker: self.accepted_marker.project_into(),
        }
    }
}

impl ProjectInto<schema::MirrorPayload> for signal_upgrade::MirrorPayload {
    fn project_into(self) -> schema::MirrorPayload {
        schema::MirrorPayload {
            component: self.component,
            source_version: self.source_version.project_into(),
            target_version: self.target_version.project_into(),
            kind: self.kind,
            payload: self.payload,
        }
    }
}

impl ProjectInto<schema::DivergenceReason> for signal_upgrade::DivergenceReason {
    fn project_into(self) -> schema::DivergenceReason {
        match self {
            Self::NotRepresentable => schema::DivergenceReason::NotRepresentable,
            Self::TargetUnavailable => schema::DivergenceReason::TargetUnavailable,
            Self::TargetRejected => schema::DivergenceReason::TargetRejected,
        }
    }
}

impl ProjectInto<signal_upgrade::DivergenceReason> for schema::DivergenceReason {
    fn project_into(self) -> signal_upgrade::DivergenceReason {
        match self {
            Self::NotRepresentable => signal_upgrade::DivergenceReason::NotRepresentable,
            Self::TargetUnavailable => signal_upgrade::DivergenceReason::TargetUnavailable,
            Self::TargetRejected => signal_upgrade::DivergenceReason::TargetRejected,
        }
    }
}

impl ProjectInto<schema::DivergencePayload> for signal_upgrade::DivergencePayload {
    fn project_into(self) -> schema::DivergencePayload {
        schema::DivergencePayload {
            component: self.component,
            source_version: self.source_version.project_into(),
            target_version: self.target_version.project_into(),
            reason: self.reason.project_into(),
            kind: self.kind,
            payload: self.payload,
        }
    }
}

impl ProjectInto<schema::RecoveryRequest> for signal_upgrade::RecoveryRequest {
    fn project_into(self) -> schema::RecoveryRequest {
        schema::RecoveryRequest {
            component: self.component,
            failure_identifier: self.failure_identifier,
        }
    }
}

impl ProjectInto<signal_upgrade::InspectionReported> for schema::InspectionReported {
    fn project_into(self) -> signal_upgrade::InspectionReported {
        signal_upgrade::InspectionReported::new(
            self.into_payload()
                .into_iter()
                .map(|migration| signal_upgrade::SupportedMigration {
                    component: migration.component,
                    source: migration.source.project_into(),
                    target: migration.target.project_into(),
                    identifier: migration.identifier,
                })
                .collect(),
        )
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

impl ProjectInto<signal_upgrade::HandoverAcceptance> for schema::HandoverAcceptance {
    fn project_into(self) -> signal_upgrade::HandoverAcceptance {
        signal_upgrade::HandoverAcceptance::new(self.into_payload().project_into())
    }
}

impl ProjectInto<signal_upgrade::HandoverFinalization> for schema::HandoverFinalization {
    fn project_into(self) -> signal_upgrade::HandoverFinalization {
        signal_upgrade::HandoverFinalization::new(self.into_payload().project_into())
    }
}

impl ProjectInto<signal_upgrade::MirrorAcknowledgement> for schema::MirrorAcknowledgement {
    fn project_into(self) -> signal_upgrade::MirrorAcknowledgement {
        signal_upgrade::MirrorAcknowledgement {
            component: self.component,
            mirrored_write_count: self.mirrored_write_count,
        }
    }
}

impl ProjectInto<signal_upgrade::DivergenceAcknowledgement> for schema::DivergenceAcknowledgement {
    fn project_into(self) -> signal_upgrade::DivergenceAcknowledgement {
        signal_upgrade::DivergenceAcknowledgement {
            component: self.component,
            divergence_identifier: self.divergence_identifier,
        }
    }
}

impl ProjectInto<signal_upgrade::RecoveryResult> for schema::RecoveryResult {
    fn project_into(self) -> signal_upgrade::RecoveryResult {
        signal_upgrade::RecoveryResult {
            component: self.component,
            recovered: self.recovered,
        }
    }
}

impl ProjectInto<signal_upgrade::HandoverRejectionReason> for schema::HandoverRejectionReason {
    fn project_into(self) -> signal_upgrade::HandoverRejectionReason {
        match self {
            Self::SchemaMismatch => signal_upgrade::HandoverRejectionReason::SchemaMismatch,
            Self::StateSequenceAdvanced => {
                signal_upgrade::HandoverRejectionReason::StateSequenceAdvanced
            }
            Self::AlreadyInHandover => signal_upgrade::HandoverRejectionReason::AlreadyInHandover,
            Self::NotReady => signal_upgrade::HandoverRejectionReason::NotReady,
        }
    }
}

impl ProjectInto<signal_upgrade::HandoverRejection> for schema::HandoverRejection {
    fn project_into(self) -> signal_upgrade::HandoverRejection {
        signal_upgrade::HandoverRejection {
            component: self.component,
            reason: self.reason.project_into(),
        }
    }
}

impl ProjectInto<Output> for schema::Output {
    fn project_into(self) -> Output {
        match self {
            schema::Output::InspectionReported(payload) => {
                Output::InspectionReported(payload.project_into())
            }
            schema::Output::UpgradeCompleted(payload) => {
                Output::UpgradeCompleted(payload.project_into())
            }
            schema::Output::UpgradeRejected(payload) => {
                Output::UpgradeRejected(payload.project_into())
            }
            schema::Output::Reported(payload) => Output::Reported(payload.project_into()),
            schema::Output::HandoverMarker(payload) => {
                Output::HandoverMarker(payload.project_into())
            }
            schema::Output::HandoverAccepted(payload) => {
                Output::HandoverAccepted(payload.project_into())
            }
            schema::Output::HandoverFinalized(payload) => {
                Output::HandoverFinalized(payload.project_into())
            }
            schema::Output::MirrorAcknowledged(payload) => {
                Output::MirrorAcknowledged(payload.project_into())
            }
            schema::Output::DivergenceAcknowledged(payload) => {
                Output::DivergenceAcknowledged(payload.project_into())
            }
            schema::Output::RecoveryCompleted(payload) => {
                Output::RecoveryCompleted(payload.project_into())
            }
            schema::Output::HandoverRejected(payload) => {
                Output::HandoverRejected(payload.project_into())
            }
            schema::Output::RequestUnimplemented(payload) => {
                Output::request_unimplemented(payload.into_payload().project_into())
            }
            schema::Output::Registered(_)
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

impl ProjectInto<UnimplementedReason> for schema::UnimplementedReason {
    fn project_into(self) -> UnimplementedReason {
        match self {
            Self::NotBuiltYet => UnimplementedReason::NotBuiltYet,
            Self::IntegrationNotLanded => UnimplementedReason::IntegrationNotLanded,
        }
    }
}
