//! Watches port connection state over a few seconds, to spot a cable that keeps
//! dropping and reconnecting. Each sample is one read of every port.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PortSample {
    pub t_ms: u64,
    pub port_id: String,
    pub connected: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CyclingAlert {
    pub port_id: String,
    pub connects: usize,
    pub window_ms: u64,
    pub per_minute: f64,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PortWatch {
    pub duration_ms: u64,
    pub sample_interval_ms: u64,
    pub samples: Vec<PortSample>,
    pub alerts: Vec<CyclingAlert>,
    pub summary: String,
}

/// Connects needed inside one window before a port counts as cycling. A person
/// plugs in once or twice in ten seconds, so six is well above normal use.
pub const CYCLE_CONNECTS: usize = 6;
pub const CYCLE_WINDOW_MS: u64 = 10_000;

pub fn analyze(samples: &[PortSample], duration_ms: u64, interval_ms: u64) -> PortWatch {
    let mut ports: Vec<&str> = samples.iter().map(|s| s.port_id.as_str()).collect();
    ports.sort_unstable();
    ports.dedup();

    let mut alerts = Vec::new();
    for port in ports {
        let mut rows: Vec<&PortSample> = samples.iter().filter(|s| s.port_id == port).collect();
        rows.sort_by_key(|s| s.t_ms);
        // Times at which the port went from empty to connected.
        let connects: Vec<u64> = rows
            .windows(2)
            .filter(|w| !w[0].connected && w[1].connected)
            .map(|w| w[1].t_ms)
            .collect();
        let most = (0..connects.len())
            .map(|i| {
                connects[i..]
                    .iter()
                    .take_while(|&&t| t - connects[i] <= CYCLE_WINDOW_MS)
                    .count()
            })
            .max()
            .unwrap_or(0);
        if most >= CYCLE_CONNECTS {
            alerts.push(CyclingAlert {
                port_id: port.to_string(),
                connects: most,
                window_ms: CYCLE_WINDOW_MS,
                per_minute: most as f64 * 60_000.0 / CYCLE_WINDOW_MS as f64,
                message: format!(
                    "{port} connected {most} times in {} seconds. Unplug the cable now: repeated power cycling can wear the port and the cable.",
                    CYCLE_WINDOW_MS / 1000
                ),
            });
        }
    }

    let summary = match alerts.first() {
        Some(a) => a.message.clone(),
        None => format!(
            "No port cycled in {} seconds of sampling.",
            duration_ms / 1000
        ),
    };
    PortWatch {
        duration_ms,
        sample_interval_ms: interval_ms,
        samples: samples.to_vec(),
        alerts,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One port sampled every `step` ms with the given connected pattern.
    fn series(port: &str, pattern: &[bool], step: u64) -> Vec<PortSample> {
        pattern
            .iter()
            .enumerate()
            .map(|(i, &c)| PortSample {
                t_ms: i as u64 * step,
                port_id: port.into(),
                connected: c,
            })
            .collect()
    }

    #[test]
    fn steady_port_has_no_alert() {
        let w = analyze(&series("p", &[true; 40], 250), 10_000, 250);
        assert!(w.alerts.is_empty());
        assert!(w.summary.starts_with("No port cycled"));
    }

    #[test]
    fn one_replug_is_normal_use() {
        let mut pattern = vec![true; 20];
        pattern.extend(vec![false; 4]);
        pattern.extend(vec![true; 20]);
        assert!(
            analyze(&series("p", &pattern, 250), 15_000, 250)
                .alerts
                .is_empty()
        );
    }

    #[test]
    fn rapid_cycling_is_flagged_and_named() {
        // On for 288 ms, off for 655 ms, sampled every 250 ms.
        let pattern: Vec<bool> = (0..80)
            .map(|i| (i % 4) < 2 || i % 4 == 3 && false)
            .collect();
        let w = analyze(&series("Port-USB-C@4", &pattern, 250), 20_000, 250);
        assert_eq!(w.alerts.len(), 1);
        let a = &w.alerts[0];
        assert_eq!(a.port_id, "Port-USB-C@4");
        assert!(a.connects >= CYCLE_CONNECTS);
        assert!(a.message.contains("Unplug the cable now"));
        assert!(w.summary.contains("Unplug"));
    }

    #[test]
    fn ports_are_judged_separately() {
        let mut samples = series("a", &[true; 40], 250);
        samples.extend(series(
            "b",
            &(0..40).map(|i| i % 2 == 0).collect::<Vec<_>>(),
            250,
        ));
        let w = analyze(&samples, 10_000, 250);
        assert_eq!(w.alerts.len(), 1);
        assert_eq!(w.alerts[0].port_id, "b");
    }
}
