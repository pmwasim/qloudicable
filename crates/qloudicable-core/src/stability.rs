//! Micro-dropout and USB port connection stability analytics.
//!
//! Monitored cables can suffer from intermittent contact bouncing, loose plugs,
//! or micro-dropouts that cause USB device re-enumeration or charging resets.
//! This module tracks connection events over time and calculates a cable stability score (0–100%).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Connect,
    Disconnect,
    ContractReset,
    SpeedDegraded,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PortEvent {
    pub timestamp_epoch_s: u64,
    pub port_id: String,
    pub event_type: EventType,
    pub detail: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StabilityVerdict {
    RockSolid,
    Stable,
    Jittery,
    Failing,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StabilityAnalysis {
    pub total_events: usize,
    pub dropouts: usize,
    pub renegotiations: usize,
    pub speed_drops: usize,
    pub score: u8,
    pub verdict: StabilityVerdict,
    pub summary: String,
}

pub fn analyze_stability(events: &[PortEvent]) -> StabilityAnalysis {
    let mut dropouts = 0;
    let mut renegotiations = 0;
    let mut speed_drops = 0;

    for ev in events {
        match ev.event_type {
            EventType::Disconnect => dropouts += 1,
            EventType::ContractReset => renegotiations += 1,
            EventType::SpeedDegraded => speed_drops += 1,
            EventType::Connect => {}
        }
    }

    let penalty = (dropouts * 20) + (speed_drops * 10) + (renegotiations * 5);
    let score = (100_i32 - penalty as i32).clamp(0, 100) as u8;

    let verdict = match score {
        90..=100 => StabilityVerdict::RockSolid,
        70..=89 => StabilityVerdict::Stable,
        40..=69 => StabilityVerdict::Jittery,
        _ => StabilityVerdict::Failing,
    };

    let summary = match verdict {
        StabilityVerdict::RockSolid => {
            "Rock solid connection. No micro-dropouts or contact bounce detected.".to_string()
        }
        StabilityVerdict::Stable => format!(
            "Stable connection with minor activity ({dropouts} dropouts, {renegotiations} resets)."
        ),
        StabilityVerdict::Jittery => format!(
            "Jittery connection detected ({dropouts} dropouts, {speed_drops} speed drops). Possible worn plug or loose port."
        ),
        StabilityVerdict::Failing => format!(
            "Failing link: {dropouts} micro-dropouts recorded. Cable or port contacts are unreliable and risk data corruption."
        ),
    };

    StabilityAnalysis {
        total_events: events.len(),
        dropouts,
        renegotiations,
        speed_drops,
        score,
        verdict,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_connection_is_rock_solid() {
        let events = vec![PortEvent {
            timestamp_epoch_s: 1730000000,
            port_id: "port-1".into(),
            event_type: EventType::Connect,
            detail: "Charger connected".into(),
        }];
        let res = analyze_stability(&events);
        assert_eq!(res.score, 100);
        assert_eq!(res.verdict, StabilityVerdict::RockSolid);
        assert_eq!(res.dropouts, 0);
    }

    #[test]
    fn multiple_dropouts_lower_score() {
        let events = vec![
            PortEvent {
                timestamp_epoch_s: 1730000000,
                port_id: "port-1".into(),
                event_type: EventType::Connect,
                detail: "Cable attached".into(),
            },
            PortEvent {
                timestamp_epoch_s: 1730000005,
                port_id: "port-1".into(),
                event_type: EventType::Disconnect,
                detail: "Micro-drop".into(),
            },
            PortEvent {
                timestamp_epoch_s: 1730000006,
                port_id: "port-1".into(),
                event_type: EventType::Connect,
                detail: "Reconnected".into(),
            },
            PortEvent {
                timestamp_epoch_s: 1730000010,
                port_id: "port-1".into(),
                event_type: EventType::Disconnect,
                detail: "Micro-drop 2".into(),
            },
        ];
        let res = analyze_stability(&events);
        assert_eq!(res.dropouts, 2);
        assert_eq!(res.score, 60); // 100 - (2 * 20) = 60
        assert_eq!(res.verdict, StabilityVerdict::Jittery);
    }
}
