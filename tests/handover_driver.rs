use tempfile::tempdir;
use tokio::net::UnixListener;
use upgrade::{HandoverDriver, HandoverFrameCodec, SocketPath, Target, TargetInput, VersionLabel};

use signal_upgrade::{
    ComponentName, Date, HandoverAcceptance, HandoverFinalization, HandoverMarker,
    Operation as HandoverOperation, Reply as HandoverReply, Time,
};
use version_projection::{ComponentName as HandoverComponentName, ContractVersion};

fn marker(state_sequence: u64) -> HandoverMarker {
    HandoverMarker {
        component: HandoverComponentName::new("persona-spirit"),
        schema_hash: ContractVersion::new([1; 32]),
        state_sequence,
        mirrored_write_count: 7,
        record_frontier: Some(44),
        recorded_at_date: Date::new(2026, 5, 24),
        recorded_at_time: Time::new(12, 0, 0),
    }
}

fn target(current_socket: &std::path::Path, next_socket: &std::path::Path) -> Target {
    Target::from_input(TargetInput {
        component: ComponentName::new("persona-spirit"),
        current_version: VersionLabel::new("v0.1.0"),
        next_version: VersionLabel::new("v0.1.1"),
        current_meta_socket_path: SocketPath::new("/run/persona/spirit/v0.1.0/meta.sock"),
        current_upgrade_socket_path: SocketPath::new(current_socket.to_string_lossy()),
        next_meta_socket_path: SocketPath::new("/run/persona/spirit/v0.1.1/meta.sock"),
        next_upgrade_socket_path: SocketPath::new(next_socket.to_string_lossy()),
    })
}

#[tokio::test]
async fn handover_driver_drives_current_endpoint_after_matching_next_marker() {
    let directory = tempdir().expect("tempdir");
    let current_socket = directory.path().join("current-upgrade.sock");
    let next_socket = directory.path().join("next-upgrade.sock");
    let current_listener = UnixListener::bind(&current_socket).expect("current listener");
    let next_listener = UnixListener::bind(&next_socket).expect("next listener");
    let marker = marker(42);
    let current = tokio::spawn(serve_endpoint(current_listener, marker.clone(), 3));
    let next = tokio::spawn(serve_endpoint(next_listener, marker.clone(), 1));

    let driven = HandoverDriver::from_target(target(&current_socket, &next_socket))
        .drive_current_side()
        .await
        .expect("handover succeeds");

    assert_eq!(driven.marker(), &marker);
    assert_eq!(driven.acceptance().accepted_marker, marker);
    assert_eq!(driven.finalization().finalized_marker, marker);

    let current_operations = current.await.expect("current task");
    let next_operations = next.await.expect("next task");
    assert!(matches!(
        current_operations.as_slice(),
        [
            HandoverOperation::AskHandoverMarker(_),
            HandoverOperation::ReadyToHandover(_),
            HandoverOperation::HandoverCompleted(_)
        ]
    ));
    assert!(matches!(
        next_operations.as_slice(),
        [HandoverOperation::AskHandoverMarker(_)]
    ));
}

#[tokio::test]
async fn handover_driver_rejects_next_marker_drift_before_current_readiness() {
    let directory = tempdir().expect("tempdir");
    let current_socket = directory.path().join("current-drift.sock");
    let next_socket = directory.path().join("next-drift.sock");
    let current_listener = UnixListener::bind(&current_socket).expect("current listener");
    let next_listener = UnixListener::bind(&next_socket).expect("next listener");
    let current = tokio::spawn(serve_endpoint(current_listener, marker(42), 1));
    let next = tokio::spawn(serve_endpoint(next_listener, marker(43), 1));

    let error = HandoverDriver::from_target(target(&current_socket, &next_socket))
        .drive_current_side()
        .await
        .expect_err("handover rejects stale next marker");

    assert!(matches!(
        error,
        upgrade::Error::NextHandoverMarkerMismatch {
            field: "state_sequence",
            ..
        }
    ));
    assert_eq!(current.await.expect("current task").len(), 1);
    assert_eq!(next.await.expect("next task").len(), 1);
}

async fn serve_endpoint(
    listener: UnixListener,
    marker: HandoverMarker,
    exchanges: usize,
) -> Vec<HandoverOperation> {
    let codec = HandoverFrameCodec::default();
    let mut operations = Vec::new();
    for _ in 0..exchanges {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let request = codec
            .request_from_frame(codec.read_frame(&mut stream).await.expect("read request"))
            .expect("request frame");
        let exchange = request.exchange();
        let operation = request.into_operation();
        let reply = match operation.clone() {
            HandoverOperation::AskHandoverMarker(_) => {
                HandoverReply::HandoverMarker(marker.clone())
            }
            HandoverOperation::ReadyToHandover(report) => {
                HandoverReply::HandoverAccepted(HandoverAcceptance {
                    accepted_marker: report.source_marker,
                })
            }
            HandoverOperation::HandoverCompleted(report) => {
                HandoverReply::HandoverFinalized(HandoverFinalization {
                    finalized_marker: report.accepted_marker,
                })
            }
            other => panic!("unexpected operation {other:?}"),
        };
        operations.push(operation);
        let frame = codec.reply_frame(exchange, reply);
        codec
            .write_frame(&mut stream, &frame)
            .await
            .expect("write reply");
    }
    operations
}
