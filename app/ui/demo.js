// Sample data for previewing the UI in a plain browser (?demo=mac|windows|android|empty).
// The app itself never reaches this, because it always has window.__TAURI__.
(function () {
  if (window.__TAURI__) return;
  const fixed = (v, a) => ({ kind: "fixed", min_mv: v * 1000, max_mv: v * 1000, max_ma: a * 1000, max_mw: v * a * 1000 });
  const full = { level: "full", why: "" };
  const hidden = { level: "none", why: "Windows does not let apps read this. The USB-C driver keeps it inside the kernel." };
  const base = { tool_version: "demo", notes: [], ports: [], usb_devices: [], power: null };
  const SETS = {
    mac: {
      snapshot: { ...base, platform: "macos",
        capabilities: { cable_identity: full, charger_profiles: full, link_speed: full, power_measurement: full },
        ports: [
          { id: "2:1", label: "USB-C port 1", connected: true, data_role: null, power_role: null, orientation: null, power_mode: null, pd_revision: null,
            cable: { kind: "passive", vendor_id: 0x05ac, vendor_name: "Apple, Inc.", product_id: 0x7001, xid: 0, speed: "usb2", current_a: 3, epr: false, max_watts: 60, raw_vdos: [0x180005ac, 0, 0x70010100, 0x00080020] },
            partner: { vendor_id: 0x05ac, vendor_name: "Apple, Inc.", product_id: 1, kind: "Power-only device", supports_pd: true },
            source_pdos: [fixed(5, 3), fixed(9, 3), fixed(15, 3), fixed(20, 4.7)], active_pdo: fixed(20, 3), transports: ["CC"] },
          { id: "2:2", label: "USB-C port 2", connected: false, data_role: null, power_role: null, orientation: null, power_mode: null, pd_revision: null, cable: null, partner: null, source_pdos: [], active_pdo: null, transports: [] },
        ],
        usb_devices: [{ vendor_id: 0x0781, product_id: 0x5581, vendor_name: "SanDisk Corp.", product_name: "Ultra Fit", usb_version: "3.20", speed_mbps: 5000, capable_mbps: 5000, port_id: null, is_hub: false }],
        power: { state: "charging", external_power: true, voltage_mv: 12600, current_ma: 3400, milliwatts: 42840, battery_percent: 54, source: "AppleSmartBattery" },
      },
      findings: [
        { port: "2:1", severity: "warn", code: "cable_limits_charging", title: "Cable is limiting charging speed", detail: "The cable's chip states a 60 W limit. The charger can supply 94 W. A higher-rated cable would allow more.", evidence: "reported" },
        { port: null, severity: "info", code: "measured_charge_rate", title: "Battery is charging at about 42.8 W", detail: "Measured at the battery, so it reads lower than the charger's rating.", evidence: "measured" },
        { port: null, severity: "good", code: "link_at_device_max", title: "Ultra Fit is at full speed (5 Gbps)", detail: "That is the fastest this device supports, so the cable is not holding it back.", evidence: "reported" },
      ],
    },
    windows: {
      snapshot: { ...base, platform: "windows",
        capabilities: { cable_identity: hidden, charger_profiles: hidden, link_speed: full, power_measurement: { level: "partial", why: "Measured at the battery, so it reads below the charger's rating." } },
        ports: [{ id: "hub0-3", label: "USB-C port (hub 0, port 3)", connected: true, data_role: null, power_role: null, orientation: null, power_mode: null, pd_revision: null, cable: null, partner: null, source_pdos: [], active_pdo: null, transports: [] }],
        usb_devices: [{ vendor_id: 0x0bc2, product_id: 0xab38, vendor_name: "Seagate RSS LLC", product_name: null, usb_version: "2.10", speed_mbps: 480, capable_mbps: 10000, port_id: "hub0-3", is_hub: false }],
        power: { state: "charging", external_power: true, voltage_mv: 16800, current_ma: 1500, milliwatts: 25200, battery_percent: null, source: "Windows battery driver" },
      },
      findings: [
        { port: "hub0-3", severity: "warn", code: "link_below_device", title: "Seagate RSS LLC is running slower than it can", detail: "It supports 10 Gbps but is running at 480 Mbps. The cause is usually the cable, a hub, or the port. Try a known fast cable in another port to tell which.", evidence: "inferred" },
        { port: null, severity: "info", code: "measured_charge_rate", title: "Battery is charging at about 25.2 W", detail: "Measured at the battery, so it reads lower than the charger's rating. This system does not reveal what the charger or cable are rated for, so run the charge test with a known good cable and compare.", evidence: "measured" },
      ],
    },
    android: {
      snapshot: { ...base, platform: "android",
        capabilities: { cable_identity: { level: "none", why: "Android keeps this from normal apps." }, charger_profiles: { level: "none", why: "Android keeps this from normal apps." }, link_speed: { level: "none", why: "Android lists USB devices but not their current speed." }, power_measurement: { level: "partial", why: "Measured at the battery. Accuracy varies by phone." } },
        power: { state: "charging", external_power: true, voltage_mv: 4210, current_ma: 3300, milliwatts: 13893, battery_percent: 41, source: "BatteryManager (ac)" },
      },
      findings: [{ port: null, severity: "info", code: "measured_charge_rate", title: "Battery is charging at about 13.9 W", detail: "Measured at the battery, so it reads lower than the charger's rating. This system does not reveal what the charger or cable are rated for, so run the charge test with a known good cable and compare.", evidence: "measured" }],
    },
    empty: {
      snapshot: { ...base, platform: "linux",
        capabilities: { cable_identity: { level: "none", why: "This computer has no USB-C ports that report to Linux." }, charger_profiles: { level: "none", why: "This computer has no USB-C ports that report to Linux." }, link_speed: full, power_measurement: { level: "none", why: "No battery with a power reading was found." } },
      },
      findings: [],
    },
  };
  const pick = new URLSearchParams(location.search).get("demo") || "mac";
  window.QloudiCableDemo = {
    invoke: async (cmd, args) => {
      const set = SETS[pick] || SETS.mac;
      if (cmd === "report") return set;
      if (cmd === "power_sample") return set.snapshot.power ? set.snapshot.power.milliwatts : null;
      if (cmd === "charge_result") {
        const w = args.watts;
        return { samples: w.length, avg_w: w[0], min_w: w[0], max_w: w[0], stability: "steady", summary: `Steady at about ${w[0].toFixed(1)} W with no dropouts.` };
      }
      if (cmd === "virtual_test") {
        const p = args.profile;
        const isDamaged = p === "counterfeit_or_damaged";
        const is240w = p === "apple_woven_epr_240w" || p === "usb4_gen3x2_40gbps_240w";
        return {
          profile: p,
          target: args.target,
          negotiated_watts: isDamaged ? 15 : is240w ? 140 : 60,
          negotiated_mbps: p === "usb4_gen3x2_40gbps_240w" ? 40000 : p === "usb3_gen2_10gbps_100w" ? 10000 : 480,
          cable_is_power_bottleneck: !is240w && args.target === "charger_140w",
          cable_is_data_bottleneck: p === "apple_woven_epr_240w" && args.target === "nvme_storage_10gbps",
          estimated_loop_resistance: {
            milliohms: isDamaged ? 480 : 105,
            confidence: 0.95,
            rating: isDamaged ? "High" : "Nominal",
            note: isDamaged ? "Excessive contact resistance detected." : "Optimal USB-IF loop resistance."
          },
          thermal_dissipation_watts: isDamaged ? 3.4 : 0.8,
          thermal_risk: isDamaged,
          stability_score: isDamaged ? 45 : 100,
          stability_verdict: isDamaged ? "Degraded" : "RockSolid",
          report: {
            tool_version: "demo",
            platform: "demo",
            findings: []
          }
        };
      }
      if (cmd === "qa_audit") {
        return {
          total_checks: 4,
          passed_checks: 4,
          pass_rate_pct: 100.0,
          certified: true,
          audit_notes: [
            "Port ID mapping valid",
            "Power telemetry within USB PD 240W spec",
            "Findings properly prioritized by severity",
            "E-Marker VDO payloads conform to USB PD structures"
          ]
        };
      }
      if (cmd === "evolution_status") {
        return {
          env: "development",
          circuit_breaker: { failure_threshold: 3, failure_count: 0, is_tripped: false, rollback_mode: false },
          flags: {
            virtual_cable_sandbox: { name: "virtual_cable_sandbox", description: "Interactive simulation sandbox", enabled: true, rollout_pct: 100, allowed_envs: ["development", "production"] },
            dynamic_resistance_meter: { name: "dynamic_resistance_meter", description: "Live milliohm R_loop calculation", enabled: true, rollout_pct: 100, allowed_envs: ["development", "production"] },
            predictive_dropout_analysis: { name: "predictive_dropout_analysis", description: "Machine learning jitter heuristics", enabled: true, rollout_pct: 50, allowed_envs: ["development"] }
          },
          active_variants: { "virtual_cable_sandbox:host": "treatment_a" }
        };
      }
      if (cmd === "trip_circuit_breaker") {
        return {
          env: "development",
          circuit_breaker: { failure_threshold: 3, failure_count: 3, is_tripped: true, rollback_mode: true },
          flags: {},
          active_variants: {}
        };
      }
      if (cmd === "write_test") return { summary: "Wrote at 1840 Mbps." };
      throw new Error("unknown command");
    },
  };
})();
