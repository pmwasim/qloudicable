use qloudicable_backend::{collect, tests_active};
use qloudicable_core::{
    bench, diagnose, render, resistance, stability, test_matrix,
    virtual_engine::{VirtualCableProfile, VirtualTarget, VirtualTester},
};
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HELP: &str = "QloudiCable by qloudihub.com: what can this USB-C cable actually do?

USAGE:
  qloudicable                     Report on every port and device
  qloudicable --json              Same report as JSON
  qloudicable --watch             Reprint whenever the report changes (Ctrl+C to stop)
  qloudicable --charge-test [S]   Sample charging power for S seconds (default 30)
  qloudicable --resistance [S]    Sample dynamic load steps to estimate loop resistance (default 15)
  qloudicable --sentinel [S]      Monitor connection stability & micro-dropouts for S seconds (default 30)
  qloudicable --matrix-test       Run synthetic hardware scenario simulation matrix
  qloudicable --virtual-test [P]  Simulate a virtual cable against chargers/devices (default: all profiles)
  qloudicable --write-test DIR    Time a 256 MB write to DIR, then delete the test file
  qloudicable --version
  qloudicable --help
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        None => {
            print!("{}", render::text(&diagnose(&collect())));
            0
        }
        Some("--json") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&diagnose(&collect())).expect("report serialises")
            );
            0
        }
        Some("--watch") => watch(),
        Some("--charge-test") => charge_test(args.get(1)),
        Some("--resistance") => resistance_test(args.get(1)),
        Some("--sentinel") => sentinel_monitor(args.get(1)),
        Some("--matrix-test") => matrix_test(),
        Some("--virtual-test") => virtual_test(args.get(1)),
        Some("--write-test") => write_test(args.get(1)),
        Some("--version") => {
            println!("qloudicable {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Some("--help" | "-h") => {
            print!("{HELP}");
            0
        }
        Some(other) => {
            eprintln!(
                "Unknown option: {other}

{HELP}"
            );
            2
        }
    };
    std::process::exit(code);
}

fn watch() -> i32 {
    let mut last = String::new();
    loop {
        let now = render::text(&diagnose(&collect()));
        if now != last {
            println!("{now}");
            last = now;
        }
        sleep(Duration::from_secs(2));
    }
}

fn charge_test(seconds: Option<&String>) -> i32 {
    let seconds: u64 = match seconds.map(|s| s.parse()) {
        None => 30,
        Some(Ok(n)) if n > 0 => n,
        Some(_) => {
            eprintln!("--charge-test takes a whole number of seconds");
            return 2;
        }
    };
    let mut watts = Vec::new();
    for i in 0..seconds {
        if let Some(mw) = collect().power.and_then(|p| p.charging_milliwatts()) {
            watts.push(mw as f64 / 1000.0);
        }
        if i + 1 < seconds {
            sleep(Duration::from_secs(1));
        }
    }
    let t = bench::charge_test(&watts);
    println!("{}", t.summary);
    println!(
        "Samples: {}, average {:.1} W, min {:.1} W, max {:.1} W",
        t.samples, t.avg_w, t.min_w, t.max_w
    );
    if t.samples == 0 { 1 } else { 0 }
}

fn resistance_test(seconds: Option<&String>) -> i32 {
    let seconds: u64 = match seconds.map(|s| s.parse()) {
        None => 15,
        Some(Ok(n)) if n >= 2 => n,
        Some(_) => {
            eprintln!("--resistance takes at least 2 seconds");
            return 2;
        }
    };
    println!("Sampling power states for {seconds}s to detect dynamic load steps...");
    let mut samples = Vec::new();
    for i in 0..seconds {
        let snap = collect();
        if let Some((v, i_ma)) = snap.power.and_then(|p| p.voltage_mv.zip(p.current_ma)) {
            samples.push((v, i_ma));
        }
        if i + 1 < seconds {
            sleep(Duration::from_secs(1));
        }
    }
    match resistance::estimate_from_series(&samples) {
        Some(r) => {
            println!("=== Cable DC Loop Resistance Estimate ===");
            println!(
                "Resistance: {} mΩ ({})",
                r.resistance_mohm,
                r.rating.label()
            );
            println!(
                "Load Step: {:.2}V @ {:.2}A -> {:.2}V @ {:.2}A (ΔV = {} mV, ΔI = {} mA)",
                r.v1_mv as f64 / 1000.0,
                r.i1_ma as f64 / 1000.0,
                r.v2_mv as f64 / 1000.0,
                r.i2_ma as f64 / 1000.0,
                r.delta_v_mv,
                r.delta_i_ma
            );
            0
        }
        None => {
            println!(
                "Insufficient dynamic load step observed during the {seconds}s sampling period."
            );
            println!(
                "Tip: Try starting a heavy task (e.g. video playback or build) while testing to trigger a load step."
            );
            1
        }
    }
}

fn sentinel_monitor(seconds: Option<&String>) -> i32 {
    let seconds: u64 = match seconds.map(|s| s.parse()) {
        None => 30,
        Some(Ok(n)) if n > 0 => n,
        Some(_) => {
            eprintln!("--sentinel takes a positive number of seconds");
            return 2;
        }
    };
    println!("Monitoring connection stability for {seconds}s...");
    let mut events = Vec::new();
    let mut last_connected = collect().ports.iter().any(|p| p.connected);

    let start = SystemTime::now();
    while start.elapsed().unwrap_or_default().as_secs() < seconds {
        sleep(Duration::from_millis(1000));
        let snap = collect();
        let now_connected = snap.ports.iter().any(|p| p.connected);
        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if !last_connected && now_connected {
            println!("[{now_epoch}] Cable/Device attached");
            events.push(stability::PortEvent {
                timestamp_epoch_s: now_epoch,
                port_id: "primary".into(),
                event_type: stability::EventType::Connect,
                detail: "Port connection established".into(),
            });
            last_connected = true;
        } else if last_connected && !now_connected {
            println!("[{now_epoch}] [DROP] Cable disconnected or bounced");
            events.push(stability::PortEvent {
                timestamp_epoch_s: now_epoch,
                port_id: "primary".into(),
                event_type: stability::EventType::Disconnect,
                detail: "Port connection dropped".into(),
            });
            last_connected = false;
        }
    }

    let report = stability::analyze_stability(&events);
    println!(
        "
=== Stability Sentinel Report ==="
    );
    println!(
        "Stability Score: {}/100 ({:?})",
        report.score, report.verdict
    );
    println!("{}", report.summary);
    println!(
        "Total events: {}, Dropouts: {}",
        report.total_events, report.dropouts
    );
    0
}

fn matrix_test() -> i32 {
    println!("Running Synthetic Hardware R&D Simulation Matrix...");
    let results = test_matrix::run_scenario_matrix();
    let mut all_ok = true;
    for (i, r) in results.iter().enumerate() {
        let status = if r.passed { "[PASS]" } else { "[FAIL]" };
        if !r.passed {
            all_ok = false;
        }
        println!("{} {}. {}: {}", status, i + 1, r.name, r.description);
        for f in &r.report.findings {
            println!("     - [{:?}] {}: {}", f.severity, f.title, f.detail);
        }
    }
    if all_ok {
        println!(
            "
All {} R&D simulation scenarios verified successfully.",
            results.len()
        );
        0
    } else {
        println!(
            "
Some simulation scenarios failed."
        );
        1
    }
}

fn write_test(dir: Option<&String>) -> i32 {
    let Some(dir) = dir else {
        eprintln!("--write-test needs a folder on the drive you want to test");
        return 2;
    };
    match tests_active::write_speed(Path::new(dir), 256, None) {
        Ok(t) => {
            println!("{}", t.summary);
            0
        }
        Err(e) => {
            eprintln!("Couldn't write to {dir}: {e}");
            1
        }
    }
}

fn virtual_test(profile_arg: Option<&String>) -> i32 {
    let profiles = match profile_arg.map(|s| s.to_lowercase()).as_deref() {
        Some("usb2") | Some("60w") => vec![VirtualCableProfile::Usb2Charging60W],
        Some("apple") | Some("240w") => vec![VirtualCableProfile::AppleWovenEpr240W],
        Some("usb3") | Some("10g") | Some("10gbps") => {
            vec![VirtualCableProfile::Usb3Gen2_10Gbps_100W]
        }
        Some("usb4") | Some("40g") | Some("40gbps") => {
            vec![VirtualCableProfile::Usb4Gen3x2_40Gbps_240W]
        }
        Some("tb4") | Some("thunderbolt") => vec![VirtualCableProfile::Thunderbolt4_40Gbps],
        Some("counterfeit") | Some("bad") => vec![VirtualCableProfile::CounterfeitOrDamaged],
        Some("unmarked") | Some("passive") => vec![VirtualCableProfile::UnmarkedPassive],
        _ => vec![
            VirtualCableProfile::AppleWovenEpr240W,
            VirtualCableProfile::Usb4Gen3x2_40Gbps_240W,
            VirtualCableProfile::Usb3Gen2_10Gbps_100W,
            VirtualCableProfile::Usb2Charging60W,
            VirtualCableProfile::CounterfeitOrDamaged,
            VirtualCableProfile::UnmarkedPassive,
        ],
    };

    println!("=== QloudiCable Virtual Cable Testing Engine ===");
    for prof in profiles {
        println!(
            "
--- Profile: {:?} ---",
            prof
        );
        // Test against a high-power 140W GaN charger
        let charger_res =
            VirtualTester::evaluate(&prof, &VirtualTarget::Charger { max_watts: 140 });
        let p_bottleneck = if charger_res.cable_is_power_bottleneck {
            " [CABLE LIMITS CHARGING]"
        } else {
            ""
        };
        println!(
            "  140W Charger -> Negotiated: {} W{}{}, Dissipation: {:.2} W (Risk: {})",
            charger_res.negotiated_watts.unwrap_or(0),
            p_bottleneck,
            if charger_res.thermal_risk {
                " [THERMAL RISK]"
            } else {
                ""
            },
            charger_res.thermal_dissipation_watts,
            charger_res.thermal_risk
        );

        // Test against a 10Gbps NVMe storage drive
        let ssd_res = VirtualTester::evaluate(
            &prof,
            &VirtualTarget::HighSpeedStorage {
                capable_mbps: 10_000,
            },
        );
        let d_bottleneck = if ssd_res.cable_is_data_bottleneck {
            " [CABLE THROTTLES DATA]"
        } else {
            ""
        };
        println!(
            "  10Gbps SSD   -> Link Speed: {} Mbps{}, Stability: {}/100 ({:?})",
            ssd_res.negotiated_mbps.unwrap_or(0),
            d_bottleneck,
            ssd_res.stability_score,
            ssd_res.stability_verdict
        );

        println!(
            "  Resistance   : {} mΩ ({})",
            charger_res.estimated_loop_resistance.resistance_mohm,
            charger_res.estimated_loop_resistance.rating.label()
        );
        for f in &charger_res.report.findings {
            println!("    * [{:?}] {}: {}", f.severity, f.title, f.detail);
        }
    }
    0
}
