use std::path::{Path, PathBuf};

use signal_frame::{
    ExchangeIdentifier, ExchangeLane, LaneSequence, NonEmpty, Reply, Request, SessionEpoch,
    SubReply,
};
use signal_upgrade::{
    CompletionReport, ComponentName, Frame as HandoverFrame, FrameBody as HandoverFrameBody,
    HandoverAcceptance, HandoverFinalization, HandoverMarker, Input as HandoverOperation,
    MarkerRequest, Output as HandoverReply, ReadinessReport, RecoveryRequest, RecoveryResult,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::error::{Error, Result};

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct VersionLabel(String);

impl VersionLabel {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&meta_signal_upgrade::VersionLabel> for VersionLabel {
    fn from(value: &meta_signal_upgrade::VersionLabel) -> Self {
        Self::new(value.payload().clone())
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SocketPath(String);

impl SocketPath {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn as_path(&self) -> &Path {
        Path::new(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    component: ComponentName,
    current_version: VersionLabel,
    next_version: VersionLabel,
    current_meta_socket_path: SocketPath,
    current_upgrade_socket_path: SocketPath,
    next_meta_socket_path: SocketPath,
    next_upgrade_socket_path: SocketPath,
}

impl Target {
    pub fn from_input(input: TargetInput) -> Self {
        Self {
            component: input.component,
            current_version: input.current_version,
            next_version: input.next_version,
            current_meta_socket_path: input.current_meta_socket_path,
            current_upgrade_socket_path: input.current_upgrade_socket_path,
            next_meta_socket_path: input.next_meta_socket_path,
            next_upgrade_socket_path: input.next_upgrade_socket_path,
        }
    }

    pub fn component(&self) -> &ComponentName {
        &self.component
    }

    pub fn current_version(&self) -> &VersionLabel {
        &self.current_version
    }

    pub fn next_version(&self) -> &VersionLabel {
        &self.next_version
    }

    pub fn current_meta_socket_path(&self) -> &SocketPath {
        &self.current_meta_socket_path
    }

    pub fn current_upgrade_socket_path(&self) -> &SocketPath {
        &self.current_upgrade_socket_path
    }

    pub fn next_meta_socket_path(&self) -> &SocketPath {
        &self.next_meta_socket_path
    }

    pub fn next_upgrade_socket_path(&self) -> &SocketPath {
        &self.next_upgrade_socket_path
    }

    pub fn prepare(&self) -> Prepared {
        let request = MarkerRequest::new(self.component.clone());
        Prepared {
            target: self.clone(),
            first_handover_operation: HandoverOperation::AskHandoverMarker(request),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetInput {
    pub component: ComponentName,
    pub current_version: VersionLabel,
    pub next_version: VersionLabel,
    pub current_meta_socket_path: SocketPath,
    pub current_upgrade_socket_path: SocketPath,
    pub next_meta_socket_path: SocketPath,
    pub next_upgrade_socket_path: SocketPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    target: Target,
    first_handover_operation: HandoverOperation,
}

impl Prepared {
    pub fn target(&self) -> &Target {
        &self.target
    }

    pub fn first_handover_operation(&self) -> &HandoverOperation {
        &self.first_handover_operation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoverEndpoint {
    path: PathBuf,
}

impl HandoverEndpoint {
    pub fn from_socket_path(path: &SocketPath) -> Self {
        Self {
            path: PathBuf::from(path.as_str()),
        }
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn as_path(&self) -> &Path {
        self.path.as_path()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandoverFrameCodec {
    maximum_frame_bytes: usize,
}

impl HandoverFrameCodec {
    pub const fn new(maximum_frame_bytes: usize) -> Self {
        Self {
            maximum_frame_bytes,
        }
    }

    pub async fn read_frame(&self, stream: &mut UnixStream) -> Result<HandoverFrame> {
        let mut prefix = [0_u8; 4];
        stream.read_exact(&mut prefix).await?;
        let length = u32::from_be_bytes(prefix) as usize;
        if length > self.maximum_frame_bytes {
            return Err(Error::DaemonFrameTooLarge { bytes: length });
        }

        let mut bytes = Vec::with_capacity(4 + length);
        bytes.extend_from_slice(&prefix);
        bytes.resize(4 + length, 0);
        stream.read_exact(&mut bytes[4..]).await?;

        Ok(HandoverFrame::decode_length_prefixed(&bytes)?)
    }

    pub async fn write_frame(&self, stream: &mut UnixStream, frame: &HandoverFrame) -> Result<()> {
        let bytes = frame.encode_length_prefixed()?;
        stream.write_all(&bytes).await?;
        stream.flush().await?;
        Ok(())
    }

    pub fn request_frame(&self, operation: HandoverOperation) -> HandoverFrame {
        HandoverFrame::new(HandoverFrameBody::Request {
            exchange: self.exchange(),
            request: Request::from_payload(operation),
        })
    }

    pub fn reply_frame(&self, exchange: ExchangeIdentifier, reply: HandoverReply) -> HandoverFrame {
        HandoverFrame::new(HandoverFrameBody::Reply {
            exchange,
            reply: Reply::committed(NonEmpty::single(SubReply::Ok(reply))),
        })
    }

    pub fn request_from_frame(&self, frame: HandoverFrame) -> Result<ReceivedHandoverRequest> {
        match frame.into_body() {
            HandoverFrameBody::Request { exchange, request } => {
                let mut operations = request.payloads.into_vec();
                if operations.len() != 1 {
                    return Err(Error::UnexpectedSignalFrame {
                        got: format!(
                            "upgrade endpoint currently accepts one operation, got {}",
                            operations.len()
                        ),
                    });
                }
                Ok(ReceivedHandoverRequest {
                    exchange,
                    operation: operations.remove(0),
                })
            }
            other => Err(Error::UnexpectedSignalFrame {
                got: format!("{other:?}"),
            }),
        }
    }

    pub fn reply_from_frame(&self, frame: HandoverFrame) -> Result<HandoverReply> {
        match frame.into_body() {
            HandoverFrameBody::Reply { reply, .. } => match reply {
                Reply::Accepted { per_operation, .. } => {
                    let mut operations = per_operation.into_vec();
                    if operations.len() != 1 {
                        return Err(Error::UnexpectedSignalFrame {
                            got: format!(
                                "upgrade client currently accepts one reply operation, got {}",
                                operations.len()
                            ),
                        });
                    }
                    match operations.remove(0) {
                        SubReply::Ok(payload) => Ok(payload),
                        other => Err(Error::UnexpectedSignalFrame {
                            got: format!("{other:?}"),
                        }),
                    }
                }
                Reply::Rejected { reason } => Err(Error::UnexpectedSignalFrame {
                    got: format!("{reason:?}"),
                }),
            },
            other => Err(Error::UnexpectedSignalFrame {
                got: format!("{other:?}"),
            }),
        }
    }

    fn exchange(&self) -> ExchangeIdentifier {
        ExchangeIdentifier::new(
            SessionEpoch::new(1),
            ExchangeLane::Connector,
            LaneSequence::first(),
        )
    }
}

impl Default for HandoverFrameCodec {
    fn default() -> Self {
        Self::new(1024 * 1024)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedHandoverRequest {
    exchange: ExchangeIdentifier,
    operation: HandoverOperation,
}

impl ReceivedHandoverRequest {
    pub fn exchange(&self) -> ExchangeIdentifier {
        self.exchange
    }

    pub fn into_operation(self) -> HandoverOperation {
        self.operation
    }
}

#[derive(Debug, Clone)]
pub struct HandoverClient {
    endpoint: HandoverEndpoint,
    codec: HandoverFrameCodec,
}

impl HandoverClient {
    pub fn new(endpoint: HandoverEndpoint) -> Self {
        Self {
            endpoint,
            codec: HandoverFrameCodec::default(),
        }
    }

    pub async fn submit(&self, operation: HandoverOperation) -> Result<HandoverReply> {
        let mut stream = UnixStream::connect(self.endpoint.as_path()).await?;
        let frame = self.codec.request_frame(operation);
        self.codec.write_frame(&mut stream, &frame).await?;
        let reply = self.codec.read_frame(&mut stream).await?;
        self.codec.reply_from_frame(reply)
    }

    pub async fn ask_marker(&self, request: MarkerRequest) -> Result<HandoverMarker> {
        match self
            .submit(HandoverOperation::AskHandoverMarker(request))
            .await?
        {
            HandoverReply::HandoverMarker(marker) => Ok(marker),
            other => Err(Error::UnexpectedSignalFrame {
                got: format!("{other:?}"),
            }),
        }
    }

    pub async fn ready_to_handover(&self, report: ReadinessReport) -> Result<HandoverAcceptance> {
        match self
            .submit(HandoverOperation::ReadyToHandover(report))
            .await?
        {
            HandoverReply::HandoverAccepted(acceptance) => Ok(acceptance),
            other => Err(Error::UnexpectedSignalFrame {
                got: format!("{other:?}"),
            }),
        }
    }

    pub async fn complete_handover(
        &self,
        report: CompletionReport,
    ) -> Result<HandoverFinalization> {
        match self
            .submit(HandoverOperation::HandoverCompleted(report))
            .await?
        {
            HandoverReply::HandoverFinalized(finalization) => Ok(finalization),
            other => Err(Error::UnexpectedSignalFrame {
                got: format!("{other:?}"),
            }),
        }
    }

    pub async fn recover_from_failure(&self, request: RecoveryRequest) -> Result<RecoveryResult> {
        match self
            .submit(HandoverOperation::RecoverFromFailure(request))
            .await?
        {
            HandoverReply::RecoveryCompleted(result) => Ok(result),
            other => Err(Error::UnexpectedSignalFrame {
                got: format!("{other:?}"),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrivenHandover {
    marker: HandoverMarker,
    acceptance: HandoverAcceptance,
    finalization: HandoverFinalization,
}

impl DrivenHandover {
    pub fn new(
        marker: HandoverMarker,
        acceptance: HandoverAcceptance,
        finalization: HandoverFinalization,
    ) -> Self {
        Self {
            marker,
            acceptance,
            finalization,
        }
    }

    pub fn marker(&self) -> &HandoverMarker {
        &self.marker
    }

    pub fn acceptance(&self) -> &HandoverAcceptance {
        &self.acceptance
    }

    pub fn finalization(&self) -> &HandoverFinalization {
        &self.finalization
    }
}

#[derive(Debug, Clone)]
pub struct HandoverDriver {
    target: Target,
    current: HandoverClient,
    next: HandoverClient,
}

impl HandoverDriver {
    pub fn from_target(target: Target) -> Self {
        let current = HandoverClient::new(HandoverEndpoint::from_socket_path(
            target.current_upgrade_socket_path(),
        ));
        let next = HandoverClient::new(HandoverEndpoint::from_socket_path(
            target.next_upgrade_socket_path(),
        ));
        Self {
            target,
            current,
            next,
        }
    }

    pub async fn drive_current_side(&self) -> Result<DrivenHandover> {
        let component = self.target.component().clone();
        let marker = self
            .current
            .ask_marker(MarkerRequest::new(component.clone()))
            .await?;
        let next_marker = self
            .next
            .ask_marker(MarkerRequest::new(component.clone()))
            .await?;
        Self::ensure_next_marker_matches(&marker, &next_marker)?;
        let acceptance = self
            .current
            .ready_to_handover(ReadinessReport {
                component: component.clone(),
                source_marker: marker.clone(),
            })
            .await?;
        let accepted_marker = acceptance_marker(&acceptance).clone();
        let finalization = match self
            .current
            .complete_handover(CompletionReport {
                component: component.clone(),
                accepted_marker: accepted_marker.clone(),
            })
            .await
        {
            Ok(finalization) => finalization,
            Err(error) => {
                let _ = self
                    .current
                    .recover_from_failure(RecoveryRequest {
                        component,
                        failure_identifier: accepted_marker.state_sequence,
                    })
                    .await;
                return Err(error);
            }
        };
        Ok(DrivenHandover::new(marker, acceptance, finalization))
    }

    fn ensure_next_marker_matches(source: &HandoverMarker, next: &HandoverMarker) -> Result<()> {
        Self::ensure_marker_field(
            "component",
            source.component.payload(),
            next.component.payload(),
        )?;
        Self::ensure_marker_field(
            "state_sequence",
            source.state_sequence.to_string(),
            next.state_sequence.to_string(),
        )?;
        Self::ensure_marker_field(
            "mirrored_write_count",
            source.mirrored_write_count.to_string(),
            next.mirrored_write_count.to_string(),
        )?;
        Self::ensure_marker_field(
            "record_frontier",
            format!("{:?}", source.record_frontier),
            format!("{:?}", next.record_frontier),
        )?;
        Ok(())
    }

    fn ensure_marker_field(
        field: &'static str,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Result<()> {
        let expected = expected.into();
        let actual = actual.into();
        if expected == actual {
            Ok(())
        } else {
            Err(Error::NextHandoverMarkerMismatch {
                field,
                expected,
                actual,
            })
        }
    }
}

fn acceptance_marker(acceptance: &HandoverAcceptance) -> &HandoverMarker {
    acceptance.payload()
}
