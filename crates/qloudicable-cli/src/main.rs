use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use qloudicable_backend::{collect, tests_active};
use qloudicable_core::{bench, diagnose, render};

const HELP: &str = "QloudiCable by qloudihub.com: what can this USB-C cable actually do?

USAGE:
  qloudicable                     Report on every port and device
  qloudicable --json              Same report as JSON
  qloudicable --watch             Reprint whenever the report changes (Ctrl+C to stop)
  qloudicable --charge-test [S]   Sample charging power for S seconds (default 30)
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
            println!("{}", serde_json::to_string_pretty(&diagnose(&collect())).expect("report serialises"));
            0
        }
        Some("--watch") => watch(),
        Some("--charge-test") => charge_test(args.get(1)),
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
            eprintln!("Unknown option: {other}\n\n{HELP}");
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
    println!("Samples: {}, average {:.1} W, min {:.1} W, max {:.1} W", t.samples, t.avg_w, t.min_w, t.max_w);
    if t.samples == 0 { 1 } else { 0 }
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
