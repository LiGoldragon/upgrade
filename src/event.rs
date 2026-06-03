use meta_signal_upgrade::{ForceFlip, Quarantine, Rollback};
use signal_upgrade::{ComponentName, HandoverMarker};
use version_projection::{ComponentName as ProjectionComponentName, ContractVersion};

use crate::handover::{SocketPath, Target, VersionLabel};

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone, PartialEq, Eq)]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext,
    __C::Error: rkyv::rancor::Source
)))]
pub struct PreparedEvent {
    component: ComponentName,
    current_version: VersionLabel,
    next_version: VersionLabel,
    current_owner_socket_path: SocketPath,
    current_upgrade_socket_path: SocketPath,
    next_owner_socket_path: SocketPath,
    next_upgrade_socket_path: SocketPath,
}

impl PreparedEvent {
    pub fn from_target(target: &Target) -> Self {
        Self {
            component: target.component().clone(),
            current_version: target.current_version().clone(),
            next_version: target.next_version().clone(),
            current_owner_socket_path: target.current_owner_socket_path().clone(),
            current_upgrade_socket_path: target.current_upgrade_socket_path().clone(),
            next_owner_socket_path: target.next_owner_socket_path().clone(),
            next_upgrade_socket_path: target.next_upgrade_socket_path().clone(),
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
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone, PartialEq, Eq)]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext,
    __C::Error: rkyv::rancor::Source
)))]
pub enum ActiveVersionChangeSource {
    HandoverMarker {
        commit_sequence: u64,
    },
    ForceFlip {
        reason: meta_signal_upgrade::ForceReason,
    },
    Rollback {
        reason: meta_signal_upgrade::RollbackReason,
    },
}

impl ActiveVersionChangeSource {
    pub fn commit_sequence(&self) -> Option<u64> {
        match self {
            Self::HandoverMarker { commit_sequence } => Some(*commit_sequence),
            Self::ForceFlip { .. } | Self::Rollback { .. } => None,
        }
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone, PartialEq, Eq)]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext,
    __C::Error: rkyv::rancor::Source
)))]
pub struct ActiveVersionChanged {
    component: ComponentName,
    active_version: VersionLabel,
    schema_hash: ContractVersion,
    source: ActiveVersionChangeSource,
}

impl ActiveVersionChanged {
    pub fn from_marker(target: &Target, marker: &HandoverMarker) -> Self {
        Self {
            component: target.component().clone(),
            active_version: target.next_version().clone(),
            schema_hash: marker.schema_hash,
            source: ActiveVersionChangeSource::HandoverMarker {
                commit_sequence: marker.commit_sequence,
            },
        }
    }

    pub fn from_force_flip(order: &ForceFlip) -> Self {
        Self {
            component: component_from_projection(&order.component),
            active_version: VersionLabel::from(&order.target_version.label),
            schema_hash: order.target_version.contract_version,
            source: ActiveVersionChangeSource::ForceFlip {
                reason: order.reason,
            },
        }
    }

    pub fn from_rollback(order: &Rollback) -> Self {
        Self {
            component: component_from_projection(&order.component),
            active_version: VersionLabel::from(&order.restore_version.label),
            schema_hash: order.restore_version.contract_version,
            source: ActiveVersionChangeSource::Rollback {
                reason: order.reason,
            },
        }
    }

    pub fn component(&self) -> &ComponentName {
        &self.component
    }

    pub fn active_version(&self) -> &VersionLabel {
        &self.active_version
    }

    pub fn schema_hash(&self) -> ContractVersion {
        self.schema_hash
    }

    pub fn source(&self) -> &ActiveVersionChangeSource {
        &self.source
    }

    pub fn commit_sequence(&self) -> Option<u64> {
        self.source.commit_sequence()
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone, PartialEq, Eq)]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext,
    __C::Error: rkyv::rancor::Source
)))]
pub struct VersionQuarantined {
    component: ComponentName,
    version: VersionLabel,
    schema_hash: ContractVersion,
    reason: meta_signal_upgrade::QuarantineReason,
}

impl VersionQuarantined {
    pub fn from_quarantine(order: &Quarantine) -> Self {
        Self {
            component: component_from_projection(&order.component),
            version: VersionLabel::from(&order.version.label),
            schema_hash: order.version.contract_version,
            reason: order.reason,
        }
    }

    pub fn component(&self) -> &ComponentName {
        &self.component
    }

    pub fn version(&self) -> &VersionLabel {
        &self.version
    }

    pub fn schema_hash(&self) -> ContractVersion {
        self.schema_hash
    }

    pub fn reason(&self) -> meta_signal_upgrade::QuarantineReason {
        self.reason
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ActiveVersion {
    component: ComponentName,
    active_version: VersionLabel,
    schema_hash: ContractVersion,
    source: ActiveVersionChangeSource,
}

impl ActiveVersion {
    pub fn new(
        component: ComponentName,
        active_version: VersionLabel,
        schema_hash: ContractVersion,
        source: ActiveVersionChangeSource,
    ) -> Self {
        Self {
            component,
            active_version,
            schema_hash,
            source,
        }
    }

    pub fn from_change(change: &ActiveVersionChanged) -> Self {
        Self::new(
            change.component.clone(),
            change.active_version.clone(),
            change.schema_hash,
            change.source.clone(),
        )
    }

    pub fn component(&self) -> &ComponentName {
        &self.component
    }

    pub fn active_version(&self) -> &VersionLabel {
        &self.active_version
    }

    pub fn schema_hash(&self) -> ContractVersion {
        self.schema_hash
    }

    pub fn source(&self) -> &ActiveVersionChangeSource {
        &self.source
    }

    pub fn commit_sequence(&self) -> Option<u64> {
        self.source.commit_sequence()
    }
}

fn component_from_projection(component: &ProjectionComponentName) -> ComponentName {
    ComponentName::new(component.as_str())
}
