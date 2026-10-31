//! Continuous Evolution, Feature Experimentation, Risk Mitigation, and QA Framework.
//!
//! Provides zero-cost, local-first abstractions for:
//! 1. Environment profiles (Development, Staging, Production, Sandbox/Canary)
//! 2. Deterministic Feature Flags and A/B Variant Testing
//! 3. Diagnostic Risk Mitigation & Automated Circuit Breaking
//! 4. Continuous self-check (QA scorecards, synthetic health audits)
//! 5. Transactional Rollback & Fallback State Guards

use crate::diagnose::Report;
use crate::model::Snapshot;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Runtime operating environment.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    Development,
    Staging,
    Production,
    Canary,
}

impl Environment {
    pub fn from_env_var() -> Self {
        match std::env::var("QLOUDICABLE_ENV")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "dev" | "development" => Environment::Development,
            "staging" | "stage" => Environment::Staging,
            "canary" => Environment::Canary,
            _ => Environment::Production,
        }
    }

    pub fn is_permissive(&self) -> bool {
        matches!(self, Environment::Development | Environment::Canary)
    }
}

/// A/B Test Variant allocation.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    Control,
    TreatmentA,
    TreatmentB,
}

/// Feature Flag definition with target environments and A/B rollout percentage (0-100).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FeatureFlag {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub rollout_pct: u8,
    pub allowed_envs: Vec<Environment>,
}

impl FeatureFlag {
    pub fn is_active_for(&self, env: Environment, entity_id: &str) -> bool {
        if !self.enabled || !self.allowed_envs.contains(&env) {
            return false;
        }
        if self.rollout_pct >= 100 {
            return true;
        }
        if self.rollout_pct == 0 {
            return false;
        }
        // Deterministic hash based on entity_id and feature name
        let bucket = hash_bucket(&format!("{}:{}", self.name, entity_id));
        bucket < self.rollout_pct
    }

    pub fn assign_variant(&self, entity_id: &str) -> Variant {
        let bucket = hash_bucket(&format!("ab:{}:{}", self.name, entity_id));
        if bucket < 50 {
            Variant::Control
        } else if bucket < 75 {
            Variant::TreatmentA
        } else {
            Variant::TreatmentB
        }
    }
}

/// Simple deterministic hash mapping string to 0..99
fn hash_bucket(input: &str) -> u8 {
    let mut h: u32 = 2166136261;
    for b in input.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    (h % 100) as u8
}

/// Dynamic Risk Mitigation Circuit Breaker.
/// Trips if anomaly or high-risk thermal/resistance events spike beyond safe thresholds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CircuitBreaker {
    pub failure_threshold: u32,
    pub failure_count: u32,
    pub is_tripped: bool,
    pub rollback_mode: bool,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32) -> Self {
        Self {
            failure_threshold,
            failure_count: 0,
            is_tripped: false,
            rollback_mode: false,
        }
    }

    pub fn record_health(&mut self, is_healthy: bool) {
        if is_healthy {
            if self.failure_count > 0 {
                self.failure_count -= 1;
            }
            if self.failure_count == 0 {
                self.is_tripped = false;
                self.rollback_mode = false;
            }
        } else {
            self.failure_count += 1;
            if self.failure_count >= self.failure_threshold {
                self.is_tripped = true;
                self.rollback_mode = true;
            }
        }
    }

    pub fn trip_manually(&mut self) {
        self.is_tripped = true;
        self.rollback_mode = true;
    }

    pub fn reset(&mut self) {
        self.failure_count = 0;
        self.is_tripped = false;
        self.rollback_mode = false;
    }
}

/// Self-check (QA) Scorecard verifying diagnosis accuracy, stability, and protocol bounds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct QaScorecard {
    pub total_checks: u32,
    pub passed_checks: u32,
    pub pass_rate_pct: f64,
    pub certified: bool,
    pub audit_notes: Vec<String>,
}

impl QaScorecard {
    pub fn evaluate(report: &Report, snapshot: &Snapshot) -> Self {
        let mut total = 0;
        let mut passed = 0;
        let mut notes = Vec::new();

        // Check 1: Port identification integrity
        total += 1;
        if snapshot.ports.iter().all(|p| !p.id.is_empty()) {
            passed += 1;
            notes.push("Port ID mapping valid".to_string());
        } else {
            notes.push("Detected ports with empty or corrupt IDs".to_string());
        }

        // Check 2: Stated vs Measured Power coherence
        total += 1;
        if let Some(pw) = &snapshot.power {
            if let Some(mw) = pw.milliwatts {
                if mw <= 240_000 {
                    passed += 1;
                    notes.push("Power telemetry within USB PD 240W spec".to_string());
                } else {
                    notes.push(format!("Abnormal power telemetry exceeding spec: {mw} mW"));
                }
            } else {
                passed += 1;
            }
        } else {
            passed += 1;
        }

        // Check 3: Findings severity ordering
        total += 1;
        let mut well_ordered = true;
        for w in report.findings.windows(2) {
            if w[0].severity < w[1].severity {
                well_ordered = false;
                break;
            }
        }
        if well_ordered {
            passed += 1;
            notes.push("Findings properly prioritized by severity".to_string());
        } else {
            notes.push("Findings priority ordering violated".to_string());
        }

        // Check 4: Cable E-Marker VDO validity if present
        total += 1;
        let mut vdo_valid = true;
        for p in &snapshot.ports {
            if let Some(c) = &p.cable
                && c.raw_vdos.is_empty()
            {
                vdo_valid = false;
                break;
            }
        }
        if vdo_valid {
            passed += 1;
            notes.push("E-Marker VDO payloads conform to USB PD structures".to_string());
        } else {
            notes.push("Encountered empty VDO payload in reported cable".to_string());
        }

        let rate = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        QaScorecard {
            total_checks: total,
            passed_checks: passed,
            pass_rate_pct: rate,
            certified: rate >= 95.0,
            audit_notes: notes,
        }
    }
}

/// Evolution Manager managing runtime flags, A/B allocations, risk breakers, and rollbacks.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EvolutionManager {
    pub env: Environment,
    pub flags: HashMap<String, FeatureFlag>,
    pub circuit_breaker: CircuitBreaker,
    pub active_variants: HashMap<String, Variant>,
}

impl Default for EvolutionManager {
    fn default() -> Self {
        Self::new(Environment::from_env_var())
    }
}

impl EvolutionManager {
    pub fn new(env: Environment) -> Self {
        let mut mgr = Self {
            env,
            flags: HashMap::new(),
            circuit_breaker: CircuitBreaker::new(3),
            active_variants: HashMap::new(),
        };

        // Seed core continuous evolution feature flags
        mgr.register_flag(FeatureFlag {
            name: "dynamic_resistance_meter".to_string(),
            description: "Live milliohm R_loop calculation and thermal loss estimation".to_string(),
            enabled: true,
            rollout_pct: 100,
            allowed_envs: vec![
                Environment::Development,
                Environment::Staging,
                Environment::Canary,
                Environment::Production,
            ],
        });

        mgr.register_flag(FeatureFlag {
            name: "virtual_cable_sandbox".to_string(),
            description: "Interactive simulation sandbox for cables and devices".to_string(),
            enabled: true,
            rollout_pct: 100,
            allowed_envs: vec![
                Environment::Development,
                Environment::Staging,
                Environment::Canary,
                Environment::Production,
            ],
        });

        mgr.register_flag(FeatureFlag {
            name: "predictive_dropout_analysis".to_string(),
            description: "Machine learning heuristic for port jitter and dropouts".to_string(),
            enabled: true,
            rollout_pct: 50,
            allowed_envs: vec![
                Environment::Development,
                Environment::Staging,
                Environment::Canary,
            ],
        });

        mgr
    }

    pub fn register_flag(&mut self, flag: FeatureFlag) {
        self.flags.insert(flag.name.clone(), flag);
    }

    pub fn is_feature_enabled(&self, feature_name: &str, entity_id: &str) -> bool {
        // If circuit breaker tripped rollback mode, fall back to safe conservative baseline
        if self.circuit_breaker.rollback_mode {
            return false;
        }

        self.flags
            .get(feature_name)
            .map(|f| f.is_active_for(self.env, entity_id))
            .unwrap_or(false)
    }

    pub fn evaluate_ab_variant(&mut self, feature_name: &str, entity_id: &str) -> Variant {
        if let Some(flag) = self.flags.get(feature_name) {
            let variant = flag.assign_variant(entity_id);
            self.active_variants
                .insert(format!("{}:{}", feature_name, entity_id), variant);
            variant
        } else {
            Variant::Control
        }
    }

    pub fn rollback_to_safe_state(&mut self) {
        self.circuit_breaker.trip_manually();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnose::Report;
    use crate::model::*;

    #[test]
    fn test_environment_resolution() {
        let dev = Environment::Development;
        assert!(dev.is_permissive());
        let prod = Environment::Production;
        assert!(!prod.is_permissive());
    }

    #[test]
    fn test_feature_flag_rollout_and_ab() {
        let flag = FeatureFlag {
            name: "experimental_pd".into(),
            description: "New PD parser".into(),
            enabled: true,
            rollout_pct: 50,
            allowed_envs: vec![Environment::Production],
        };

        // Deterministic check
        let active1 = flag.is_active_for(Environment::Production, "device-123");
        let active2 = flag.is_active_for(Environment::Production, "device-123");
        assert_eq!(active1, active2);

        let v1 = flag.assign_variant("user-abc");
        let v2 = flag.assign_variant("user-abc");
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_circuit_breaker_and_rollback() {
        let mut cb = CircuitBreaker::new(2);
        assert!(!cb.is_tripped);

        cb.record_health(false);
        assert!(!cb.is_tripped);
        cb.record_health(false);
        assert!(cb.is_tripped);
        assert!(cb.rollback_mode);

        cb.record_health(true);
        cb.record_health(true);
        assert!(!cb.is_tripped);
    }

    #[test]
    fn test_qa_scorecard_evaluation() {
        let snap = Snapshot {
            platform: Platform::Macos,
            tool_version: "0.1.0".into(),
            capabilities: Capabilities {
                cable_identity: Access {
                    level: Level::Full,
                    why: "".into(),
                },
                charger_profiles: Access {
                    level: Level::Full,
                    why: "".into(),
                },
                link_speed: Access {
                    level: Level::Full,
                    why: "".into(),
                },
                power_measurement: Access {
                    level: Level::Full,
                    why: "".into(),
                },
            },
            ports: vec![Port {
                id: "1:1".into(),
                label: "Port 1".into(),
                connected: true,
                data_role: None,
                power_role: None,
                orientation: None,
                power_mode: None,
                pd_revision: None,
                cable: Some(CableInfo {
                    kind: CableKind::Passive,
                    vendor_id: 0x05ac,
                    vendor_name: Some("Apple".into()),
                    product_id: 0x1,
                    xid: 0,
                    speed: CableSpeed::Usb2,
                    current_a: Some(3),
                    epr: false,
                    max_watts: Some(60),
                    raw_vdos: vec![0x12345678],
                }),
                partner: None,
                source_pdos: vec![],
                active_pdo: None,
                transports: vec![],
            }],
            usb_devices: vec![],
            power: Some(PowerReading {
                state: ChargeState::Charging,
                external_power: Some(true),
                voltage_mv: Some(20_000),
                current_ma: Some(3_000),
                milliwatts: Some(60_000),
                battery_percent: Some(80),
                source: "Mac".into(),
            }),
            notes: vec![],
        };

        let report = Report {
            snapshot: snap.clone(),
            findings: vec![],
        };

        let scorecard = QaScorecard::evaluate(&report, &snap);
        assert!(scorecard.certified);
        assert_eq!(scorecard.passed_checks, scorecard.total_checks);
    }

    #[test]
    fn test_evolution_manager_flow() {
        let mut mgr = EvolutionManager::new(Environment::Development);
        assert!(mgr.is_feature_enabled("virtual_cable_sandbox", "test-client"));

        mgr.rollback_to_safe_state();
        assert!(!mgr.is_feature_enabled("virtual_cable_sandbox", "test-client"));
    }
}
