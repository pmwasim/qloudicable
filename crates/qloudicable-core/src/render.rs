//! Plain-text report for the CLI.
use crate::diagnose::{Evidence, Report, Severity};
use crate::model::*;
use std::fmt::Write;

fn access_line(out: &mut String, name: &str, a: &Access) {
    if a.level != Level::Full {
        let state = if a.level == Level::Partial { "partial" } else { "not available" };
        let _ = writeln!(out, "  {name}: {state}. {}", a.why);
    }
}

pub fn text(r: &Report) -> String {
    let mut out = String::new();
    let s = &r.snapshot;
    let _ = writeln!(out, "QloudiCable {} on {:?}\n", s.tool_version, s.platform);

    for p in &s.ports {
        let _ = writeln!(out, "{}: {}", p.label, if p.connected { "connected" } else { "nothing connected" });
        if !p.connected {
            continue;
        }
        if let Some(c) = &p.cable {
            let vendor = c.vendor_name.as_deref().unwrap_or("unknown maker");
            let watts = c.max_watts.map_or("unstated power".to_string(), |w| format!("{w} W"));
            let _ = writeln!(out, "  Cable: {}, {watts}, {vendor}", c.speed.label());
        } else if s.capabilities.cable_identity.level == Level::Full {
            let _ = writeln!(out, "  Cable: no e-marker reported");
        }
        if !p.source_pdos.is_empty() {
            let list: Vec<String> = p.source_pdos.iter().map(|o| format!("{} W", o.max_mw / 1000)).collect();
            let _ = writeln!(out, "  Charger offers: {}", list.join(", "));
        }
    }

    let devices: Vec<_> = s.usb_devices.iter().filter(|d| !d.is_hub).collect();
    if !devices.is_empty() {
        let _ = writeln!(out, "\nUSB devices:");
        for d in devices {
            let name = d.product_name.as_deref().or(d.vendor_name.as_deref()).unwrap_or("Unnamed device");
            let speed = d.speed_mbps.map_or("speed unknown".to_string(), speed_words);
            let _ = writeln!(out, "  {name} ({:04x}:{:04x}), {speed}", d.vendor_id, d.product_id);
        }
    }

    if let Some(p) = &s.power {
        let w = p.milliwatts.map_or("no reading".to_string(), |m| format!("{:.1} W", m as f64 / 1000.0));
        let _ = writeln!(out, "\nPower: {:?}, {w} ({})", p.state, p.source);
    }

    let _ = writeln!(out, "\nFindings:");
    if r.findings.is_empty() {
        let _ = writeln!(out, "  Nothing to report.");
    }
    for f in &r.findings {
        let mark = match f.severity {
            Severity::Good => "OK  ",
            Severity::Info => "INFO",
            Severity::Warn => "WARN",
            Severity::Problem => "FAIL",
        };
        let how = match f.evidence {
            Evidence::Reported => "reported by hardware",
            Evidence::Measured => "measured",
            Evidence::Inferred => "inferred",
        };
        let _ = writeln!(out, "  {mark} {}\n       {} [{how}]", f.title, f.detail);
    }

    let c = &s.capabilities;
    let mut limits = String::new();
    access_line(&mut limits, "Cable identity", &c.cable_identity);
    access_line(&mut limits, "Charger profiles", &c.charger_profiles);
    access_line(&mut limits, "Link speed", &c.link_speed);
    access_line(&mut limits, "Power measurement", &c.power_measurement);
    if !limits.is_empty() {
        let _ = writeln!(out, "\nWhat this system can't show:\n{limits}");
    }
    out
}
