//! The exercise event ledger.
//!
//! Every timing this records is derived from an observed on-chain event, and
//! every event carries enough identity for someone else to confirm it happened:
//! a transaction signature, a slot, a message id. That is the difference between
//! *measured* and *convincing*.
//!
//! The ledger deliberately stores no durations. Durations are computed from
//! recorded timestamps at read time, so a reader can recompute them and get the
//! same answer — or a different one, and know the record was wrong.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Scenario {
    /// A dependency becomes unsafe; the chain of consequence runs to a blocked
    /// consumer.
    DependencyCompromise,
    /// Delivery stops. Nothing is published. The consumer must block anyway.
    RelayerCensorship,
}

impl Scenario {
    pub fn slug(self) -> &'static str {
        match self {
            Scenario::DependencyCompromise => "dependency-compromise",
            Scenario::RelayerCensorship => "relayer-censorship",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventKind {
    FindingRegistered,
    ImpactProjected,
    PassportInvalidated,
    SolanaCommitted,
    ConsumerBlocked,
    /// Delivery deliberately stopped — the censorship scenario.
    RelayerStopped,
    /// Expiry reached with no update delivered.
    PassportExpired,
    HyperlaneDispatched,
    DestinationReceived,
}

/// What an independent party can re-check. Anything not reconstructable is
/// marked as such rather than quietly presented as evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Evidence {
    /// Re-fetchable from any RPC for the cluster.
    SolanaTransaction { signature: String, slot: u64 },
    /// Re-fetchable from the Hyperlane explorer or the destination mailbox.
    HyperlaneMessage { message_id: String },
    /// Computed locally. **Not** independently verifiable, and labelled so.
    Local { note: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub seq: u32,
    pub kind: EventKind,
    /// Wall clock at the moment the event was observed.
    pub observed_at: i64,
    pub evidence: Evidence,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exercise {
    pub exercise_id: String,
    pub scenario: Scenario,
    pub cluster: String,
    pub started_at: i64,
    /// Program and circuit under exercise, so the record is self-describing.
    pub passport_program: String,
    pub consumer_program: String,
    pub circuit_hash: String,
    pub events: Vec<Event>,
}

impl Exercise {
    pub fn new(
        scenario: Scenario,
        cluster: &str,
        passport_program: &str,
        consumer_program: &str,
        circuit_hash: &str,
        started_at: i64,
    ) -> Self {
        Self {
            exercise_id: format!("{}-{}", scenario.slug(), started_at),
            scenario,
            cluster: cluster.to_string(),
            started_at,
            passport_program: passport_program.to_string(),
            consumer_program: consumer_program.to_string(),
            circuit_hash: circuit_hash.to_string(),
            events: Vec::new(),
        }
    }

    pub fn record(&mut self, kind: EventKind, evidence: Evidence, detail: impl Into<String>) {
        let seq = self.events.len() as u32 + 1;
        self.events.push(Event {
            seq,
            kind,
            observed_at: now(),
            evidence,
            detail: detail.into(),
        });
    }

    /// Elapsed seconds from the first to the last recorded event.
    ///
    /// Derived, never stored. A reader recomputes it from the same timestamps
    /// and either agrees or has found a problem.
    pub fn containment_secs(&self) -> Option<i64> {
        let first = self.events.first()?.observed_at;
        let last = self.events.last()?.observed_at;
        Some(last - first)
    }

    /// Events an independent party can re-check against a public source.
    pub fn verifiable_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| !matches!(e.evidence, Evidence::Local { .. }))
            .count()
    }

    pub fn save(&self, dir: &Path) -> Result<std::path::PathBuf> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.json", self.exercise_id));
        std::fs::write(&path, serde_json::to_vec_pretty(self)?)?;
        Ok(path)
    }

    pub fn load(path: &Path) -> Result<Self> {
        Ok(serde_json::from_slice(&std::fs::read(path)?)?)
    }
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ex() -> Exercise {
        Exercise::new(
            Scenario::DependencyCompromise,
            "devnet",
            "P",
            "C",
            "abc",
            1_000,
        )
    }

    #[test]
    fn local_evidence_is_not_counted_as_verifiable() {
        let mut e = ex();
        e.record(
            EventKind::ImpactProjected,
            Evidence::Local { note: "n".into() },
            "",
        );
        e.record(
            EventKind::SolanaCommitted,
            Evidence::SolanaTransaction {
                signature: "sig".into(),
                slot: 1,
            },
            "",
        );
        assert_eq!(e.events.len(), 2);
        assert_eq!(e.verifiable_count(), 1, "local events must not inflate the count");
    }

    #[test]
    fn containment_is_derived_from_recorded_timestamps() {
        let mut e = ex();
        e.record(EventKind::FindingRegistered, Evidence::Local { note: "a".into() }, "");
        e.events[0].observed_at = 100;
        e.record(EventKind::ConsumerBlocked, Evidence::Local { note: "b".into() }, "");
        e.events[1].observed_at = 142;
        assert_eq!(e.containment_secs(), Some(42));
    }

    #[test]
    fn an_empty_exercise_reports_no_containment_rather_than_zero() {
        // Zero would read as instant containment. Absent is the honest answer.
        assert_eq!(ex().containment_secs(), None);
    }

    #[test]
    fn sequence_numbers_are_assigned_in_order() {
        let mut e = ex();
        for _ in 0..3 {
            e.record(EventKind::FindingRegistered, Evidence::Local { note: "x".into() }, "");
        }
        assert_eq!(e.events.iter().map(|x| x.seq).collect::<Vec<_>>(), vec![1, 2, 3]);
    }
}
