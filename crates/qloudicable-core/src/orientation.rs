//! Guided both-ends test. The user checks each end of a cable in both plug
//! positions. Reading the four results together tells a faulty end apart from a
//! dirty contact, which a single reading can't do.
use crate::diagnose::Report;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    A,
    B,
}

/// One reading, taken while one end of the cable is in the port.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlugRun {
    pub end: End,
    /// 1 for the first position, 2 for the same end turned 180 degrees.
    pub step: u8,
    pub connected: bool,
    pub emarker_seen: bool,
    pub charge_mw: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CellState {
    Pass,
    NoEmarker,
    NotTested,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Cell {
    pub end: End,
    pub step: u8,
    pub state: CellState,
    pub charge_mw: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    Incomplete,
    AllGood,
    NoEmarkerAnywhere,
    EndFault,
    ContactFault,
    Mixed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OrientationMatrix {
    pub cells: Vec<Cell>,
    pub pattern: Pattern,
    pub verdict: String,
    pub charge_limited: bool,
}

const POSITIONS: [(End, u8); 4] = [(End::A, 1), (End::A, 2), (End::B, 1), (End::B, 2)];

pub fn position_label(end: End, step: u8) -> String {
    let flip = if step == 1 {
        "as inserted"
    } else {
        "turned 180 degrees"
    };
    format!("End {end:?}, {flip}")
}

fn other(end: End) -> End {
    match end {
        End::A => End::B,
        End::B => End::A,
    }
}

/// Reads the port state as one run. It uses only the report, so it can be tested
/// without hardware.
pub fn capture(report: &Report, end: End, step: u8) -> PlugRun {
    let connected: Vec<_> = report
        .snapshot
        .ports
        .iter()
        .filter(|p| p.connected)
        .collect();
    PlugRun {
        end,
        step,
        connected: !connected.is_empty(),
        emarker_seen: connected.iter().any(|p| p.cable.is_some()),
        charge_mw: report
            .snapshot
            .power
            .as_ref()
            .and_then(|p| p.charging_milliwatts()),
    }
}

/// Combines the runs into one matrix and a plain verdict. The latest reading for
/// each position wins, so a retest replaces an earlier result.
pub fn matrix(runs: &[PlugRun]) -> OrientationMatrix {
    let cells: Vec<Cell> = POSITIONS
        .iter()
        .map(
            |&(end, step)| match runs.iter().rev().find(|r| r.end == end && r.step == step) {
                Some(r) if r.connected => Cell {
                    end,
                    step,
                    state: if r.emarker_seen {
                        CellState::Pass
                    } else {
                        CellState::NoEmarker
                    },
                    charge_mw: r.charge_mw,
                },
                _ => Cell {
                    end,
                    step,
                    state: CellState::NotTested,
                    charge_mw: None,
                },
            },
        )
        .collect();

    let state = |end: End, step: u8| {
        cells
            .iter()
            .find(|c| c.end == end && c.step == step)
            .map_or(CellState::NotTested, |c| c.state)
    };
    let good = |e: End| state(e, 1) == CellState::Pass && state(e, 2) == CellState::Pass;
    let dead = |e: End| state(e, 1) == CellState::NoEmarker && state(e, 2) == CellState::NoEmarker;
    let uneven = |e: End| {
        matches!(
            (state(e, 1), state(e, 2)),
            (CellState::Pass, CellState::NoEmarker) | (CellState::NoEmarker, CellState::Pass)
        )
    };

    let charges: Vec<u32> = cells
        .iter()
        .filter(|c| c.state == CellState::Pass)
        .filter_map(|c| c.charge_mw)
        .collect();
    let charge_limited = match (charges.iter().max(), charges.iter().min()) {
        (Some(&hi), Some(&lo)) => hi > 0 && (lo as f64) < hi as f64 * 0.8,
        _ => false,
    };

    let (pattern, mut verdict) = if let Some(next) =
        cells.iter().find(|c| c.state == CellState::NotTested)
    {
        (
            Pattern::Incomplete,
            format!("Still to test: {}.", position_label(next.end, next.step)),
        )
    } else if cells.iter().all(|c| c.state == CellState::Pass) {
        (
            Pattern::AllGood,
            "Works from both ends and in both plug positions.".to_string(),
        )
    } else if cells.iter().all(|c| c.state == CellState::NoEmarker) {
        (
            Pattern::NoEmarkerAnywhere,
            "No end answered the e-marker check. Many cables have no e-marker and are limited to 60 W, which is normal. This is not a fault by itself.".to_string(),
        )
    } else if let Some(bad) = [End::A, End::B]
        .into_iter()
        .find(|&e| dead(e) && good(other(e)))
    {
        (
            Pattern::EndFault,
            format!(
                "End {bad:?} never answered the e-marker check, while the other end did. Both ends should answer, so suspect a fault at End {bad:?}. Confirm on another port before returning the cable."
            ),
        )
    } else if let Some(e) = [End::A, End::B].into_iter().find(|&e| uneven(e)) {
        (
            Pattern::ContactFault,
            format!(
                "End {e:?} answered in one plug position but not the other. That usually means a dirty or worn contact. Clean the plug and the port, then retest."
            ),
        )
    } else {
        (
            Pattern::Mixed,
            "The results don't match one pattern. Repeat the test. If it persists, try another port.".to_string(),
        )
    };

    if charge_limited && matches!(pattern, Pattern::AllGood | Pattern::ContactFault) {
        verdict.push_str(
            " Charging was lower in one position. That can come from the contact or the charger.",
        );
    }

    OrientationMatrix {
        cells,
        pattern,
        verdict,
        charge_limited,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Port, Snapshot};
    use crate::pd::decode_cable;

    fn run(end: End, step: u8, emarker: bool) -> PlugRun {
        PlugRun {
            end,
            step,
            connected: true,
            emarker_seen: emarker,
            charge_mw: None,
        }
    }

    fn full(a: bool, b: bool) -> Vec<PlugRun> {
        vec![
            run(End::A, 1, a),
            run(End::A, 2, a),
            run(End::B, 1, b),
            run(End::B, 2, b),
        ]
    }

    #[test]
    fn healthy_cable_passes_everywhere() {
        let m = matrix(&full(true, true));
        assert_eq!(m.pattern, Pattern::AllGood);
        assert!(m.cells.iter().all(|c| c.state == CellState::Pass));
    }

    #[test]
    fn unmarked_cable_is_not_called_faulty() {
        let m = matrix(&full(false, false));
        assert_eq!(m.pattern, Pattern::NoEmarkerAnywhere);
        assert!(m.verdict.contains("not a fault"));
    }

    #[test]
    fn one_dead_end_is_blamed_on_that_end() {
        let m = matrix(&full(false, true));
        assert_eq!(m.pattern, Pattern::EndFault);
        assert!(m.verdict.contains("suspect a fault at End A"));
    }

    #[test]
    fn one_uneven_end_is_a_contact_problem() {
        let runs = vec![
            run(End::A, 1, true),
            run(End::A, 2, false),
            run(End::B, 1, true),
            run(End::B, 2, true),
        ];
        let m = matrix(&runs);
        assert_eq!(m.pattern, Pattern::ContactFault);
        assert!(m.verdict.contains("dirty or worn contact"));
    }

    #[test]
    fn missing_positions_are_listed_first() {
        let m = matrix(&[run(End::A, 1, true)]);
        assert_eq!(m.pattern, Pattern::Incomplete);
        assert!(m.verdict.contains("End A, turned 180 degrees"));
    }

    #[test]
    fn disconnected_runs_do_not_count() {
        let mut runs = full(true, true);
        runs[1].connected = false;
        assert_eq!(matrix(&runs).pattern, Pattern::Incomplete);
    }

    #[test]
    fn latest_reading_replaces_an_earlier_one() {
        let mut runs = vec![run(End::A, 1, false)];
        runs.extend(full(true, true));
        assert_eq!(matrix(&runs).pattern, Pattern::AllGood);
    }

    #[test]
    fn lower_charge_in_one_position_is_flagged() {
        let mut runs = full(true, true);
        runs[0].charge_mw = Some(60_000);
        runs[1].charge_mw = Some(30_000);
        let m = matrix(&runs);
        assert!(m.charge_limited);
        assert!(m.verdict.contains("Charging was lower"));
    }

    #[test]
    fn capture_reads_the_report() {
        let header = (0b011 << 27) | 0x05AC;
        let cable = decode_cable(header, 0, 0, 0b10 << 5);
        let port = Port {
            id: "p".into(),
            connected: true,
            cable: Some(cable),
            ..Default::default()
        };
        let report = crate::diagnose(&Snapshot {
            ports: vec![port],
            ..Default::default()
        });
        let r = capture(&report, End::B, 2);
        assert!(r.connected && r.emarker_seen);
        assert_eq!((r.end, r.step), (End::B, 2));
    }
}
