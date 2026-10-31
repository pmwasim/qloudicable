// Device and cable strings come from hardware, so everything goes in as text.
const $ = (id) => document.getElementById(id);
function el(tag, props = {}, ...kids) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") node.className = v;
    else node[k] = v;
  }
  for (const kid of kids) if (kid !== null && kid !== undefined) node.append(kid);
  return node;
}

const tauri = window.__TAURI__;
const call = (cmd, args) => (tauri ? tauri.core.invoke(cmd, args) : window.QloudiCableDemo.invoke(cmd, args));

const SEVERITY = {
  good: { label: "Good", d: "M5 12.5 10 17.5 19 7" },
  info: { label: "Note", d: "M12 11v6M12 7.2v.1" },
  warn: { label: "Check this", d: "M12 7v6M12 16.8v.1" },
  problem: { label: "Problem", d: "M7 7l10 10M17 7 7 17" },
};
const EVIDENCE = {
  reported: "Reported by the hardware",
  measured: "Measured",
  inferred: "Worked out from the USB spec",
};

function icon(severity) {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", "24");
  svg.setAttribute("height", "24");
  svg.setAttribute("role", "img");
  svg.setAttribute("aria-label", SEVERITY[severity].label);
  const ring = document.createElementNS(ns, "circle");
  ring.setAttribute("cx", "12");
  ring.setAttribute("cy", "12");
  ring.setAttribute("r", "10.5");
  const filled = severity === "warn" || severity === "problem";
  ring.setAttribute("fill", filled ? "currentColor" : "none");
  ring.setAttribute("stroke", "currentColor");
  ring.setAttribute("stroke-width", "1.5");
  const mark = document.createElementNS(ns, "path");
  mark.setAttribute("d", SEVERITY[severity].d);
  mark.setAttribute("fill", "none");
  mark.setAttribute("stroke", filled ? "var(--panel)" : "currentColor");
  mark.setAttribute("stroke-width", "2.4");
  mark.setAttribute("stroke-linecap", "round");
  mark.setAttribute("stroke-linejoin", "round");
  svg.append(ring, mark);
  return svg;
}

const speedWords = (m) => (m === 2 ? "1.5 Mbps" : m >= 1000 ? `${m / 1000} Gbps` : `${m} Mbps`);
const watts = (mw) => `${(mw / 1000).toFixed(mw % 1000 ? 1 : 0)} W`;
const hex = (n, w) => n.toString(16).padStart(w, "0");
const CABLE_SPEED = { usb2: "USB 2.0 (480 Mbps)", usb5g: "5 Gbps", usb20g: "10 to 20 Gbps", usb40g: "40 Gbps", usb80g: "80 Gbps", reserved: "Not stated" };
const CABLE_KIND = { passive: "Passive", active: "Active", vpd: "Powered device", unknown: "Unknown" };
const STATE = { charging: "Charging", discharging: "On battery", full: "Battery full", not_charging: "Plugged in, not charging", unknown: "Unknown" };

function grid(pairs) {
  const dl = el("dl");
  for (const [k, v] of pairs) if (v) dl.append(el("dt", { textContent: k }), el("dd", { textContent: v }));
  return dl;
}

function verdict(r) {
  const s = r.snapshot;
  const worst = r.findings[0];
  const connected = s.ports.some((p) => p.connected);
  const blind = s.capabilities.cable_identity.level === "none" && !s.power;
  let title, text;
  if (worst && (worst.severity === "warn" || worst.severity === "problem")) {
    title = worst.title;
    text = r.findings.length > 1 ? `${r.findings.length} findings below, most important first.` : "Details are below.";
  } else if (worst) {
    title = worst.severity === "good" ? "No problems found" : worst.title;
    text = worst.severity === "good" ? worst.title + "." : "Details are below.";
  } else if (blind) {
    // Silence from a system that can't see cables is not a clean result.
    title = "This device can't check cables";
    text = "It doesn't reveal cable or charger details. You can still see USB devices and run the tests below.";
  } else if (connected || s.usb_devices.some((d) => !d.is_hub)) {
    title = "No problems found";
    text = "Everything this device reports looks normal.";
  } else {
    title = "Plug in a cable to start";
    text = "Connect a charger or device. This page updates by itself.";
  }
  $("verdict").replaceChildren(el("h1", { textContent: title }), el("p", { textContent: text }));
}

function findings(r) {
  $("findings-box").hidden = r.findings.length === 0;
  $("findings").replaceChildren(
    ...r.findings.map((f) =>
      el("li", { class: `finding ${f.severity}` }, icon(f.severity),
        el("div", {},
          el("strong", { textContent: f.title }),
          el("span", { class: "detail", textContent: f.detail }),
          el("br"),
          el("span", { class: "tag", textContent: EVIDENCE[f.evidence] })))
    )
  );
}

function portCard(p, identity) {
  const card = el("div", { class: "card" },
    el("h3", { textContent: p.label }),
    el("span", { class: p.connected ? "state on" : "state", textContent: p.connected ? "Connected" : "Nothing connected" }));
  if (!p.connected) return card;

  const c = p.cable;
  if (c) {
    card.append(grid([
      ["Cable speed", CABLE_SPEED[c.speed]],
      ["Cable power", c.max_watts ? `${c.max_watts} W (${c.current_a} A)` : "Not stated"],
      ["Cable type", CABLE_KIND[c.kind]],
      ["Chip maker", c.vendor_name || `Unknown (${hex(c.vendor_id, 4)})`],
      ["USB-IF ID", c.xid ? String(c.xid) : null],
    ]));
  } else if (identity === "full") {
    card.append(el("p", { class: "note", textContent: "This cable has no e-marker chip, so it is limited to 60 W and may be USB 2.0 only." }));
  } else if (identity === "partial") {
    card.append(el("p", { class: "note", textContent: "No cable chip was reported. The port driver may not read it, so this doesn't prove the cable has none." }));
  }

  const partner = p.partner;
  if (partner && (partner.vendor_name || partner.kind)) {
    card.append(grid([["Connected to", [partner.vendor_name, partner.kind].filter(Boolean).join(", ")]]));
  }
  card.append(grid([
    ["Data role", p.data_role],
    ["Power role", p.power_role],
    ["Plug side", p.orientation],
    ["In use", p.transports.length ? p.transports.join(", ") : null],
  ]));

  if (p.source_pdos.length) {
    const active = p.active_pdo && p.active_pdo.max_mv;
    card.append(el("p", { class: "note", textContent: "Charger offers" }),
      el("div", { class: "chips" }, ...p.source_pdos.map((o) => {
        const volts = o.min_mv === o.max_mv ? `${o.max_mv / 1000} V` : `${o.min_mv / 1000} to ${o.max_mv / 1000} V`;
        return el("span", { class: o.max_mv === active ? "chip active" : "chip", textContent: `${volts}, ${watts(o.max_mw)}` });
      })));
  }
  if (c) {
    card.append(el("details", {}, el("summary", { textContent: "Raw chip data" }),
      el("code", { textContent: c.raw_vdos.map((v) => "0x" + hex(v, 8)).join(" ") })));
  }
  return card;
}

function ports(s) {
  const identity = s.capabilities.cable_identity.level;
  $("ports").replaceChildren(
    ...(s.ports.length
      ? s.ports.map((p) => portCard(p, identity))
      : [el("div", { class: "card" }, el("p", { textContent: `No USB-C port details here. ${s.capabilities.cable_identity.why}` }))])
  );
}

function power(s) {
  const p = s.power;
  $("power-box").hidden = !p;
  if (!p) return;
  $("power").replaceChildren(
    el("div", { class: "big", textContent: p.milliwatts != null ? watts(p.milliwatts) : "No reading" }),
    grid([
      ["Status", STATE[p.state]],
      ["Battery", p.battery_percent != null ? `${p.battery_percent}%` : null],
      ["Voltage", p.voltage_mv ? `${(p.voltage_mv / 1000).toFixed(2)} V` : null],
      ["Current", p.current_ma ? `${(p.current_ma / 1000).toFixed(2)} A` : null],
      ["Source", p.source],
    ])
  );
}

function devices(s) {
  const list = s.usb_devices.filter((d) => !d.is_hub);
  $("devices-box").hidden = list.length === 0;
  $("devices").replaceChildren(
    ...list.map((d) => {
      const name = d.product_name || d.vendor_name || "Unnamed device";
      const sub = [d.product_name ? d.vendor_name : null, d.usb_version ? `USB ${d.usb_version}` : null, `${hex(d.vendor_id, 4)}:${hex(d.product_id, 4)}`].filter(Boolean).join(" · ");
      return el("div", { class: "row" },
        el("div", {}, name, el("span", { class: "sub", textContent: sub })),
        el("div", { class: "val", textContent: d.speed_mbps ? speedWords(d.speed_mbps) : "Speed hidden" }));
    })
  );
}

function access(s) {
  const c = s.capabilities;
  const rows = [
    ["Cable chip (e-marker)", c.cable_identity],
    ["Charger power profiles", c.charger_profiles],
    ["USB link speed", c.link_speed],
    ["Power measurement", c.power_measurement],
  ];
  const word = { full: "Yes", partial: "Partly", none: "No" };
  $("access").replaceChildren(
    ...rows.map(([name, a]) =>
      el("div", { class: "row" },
        el("div", {}, name, a.why ? el("span", { class: "sub", textContent: a.why }) : null),
        el("div", { class: "val", textContent: word[a.level] })))
  );
  $("version").textContent = s.tool_version;
  $("write-card").hidden = s.platform === "android";
}

let last = null;
let busy = false;

async function refresh() {
  if (busy) return;
  busy = true;
  try {
    const r = await call("report");
    $("error").hidden = true;
    const text = JSON.stringify(r);
    if (text !== last) {
      // Rebuilding on every poll would close open details and lose scroll.
      last = text;
      verdict(r);
      findings(r);
      ports(r.snapshot);
      power(r.snapshot);
      devices(r.snapshot);
      access(r.snapshot);
    }
  } catch (e) {
    $("error").textContent = `Couldn't read this device: ${e}`;
    $("error").hidden = false;
  } finally {
    busy = false;
  }
}

async function copyReport() {
  const b = $("copy");
  if (!last) return;
  try {
    const parsed = JSON.parse(last);
    const s = parsed.snapshot || {};
    const lines = [`=== QloudiCable ${s.tool_version || ""} on ${s.platform || ""} ===\n`];
    for (const p of s.ports || []) {
      lines.push(`${p.label}: ${p.connected ? "connected" : "nothing connected"}`);
      if (p.cable) {
        const sp = CABLE_SPEED[p.cable.speed] || p.cable.speed;
        const cur = p.cable.current_a ? ` · ${p.cable.current_a} A` : "";
        const w = p.cable.max_watts ? ` · ${p.cable.max_watts} W` : "";
        lines.push(`  Cable: ${sp}${cur}${w}`);
      }
    }
    if (s.power) {
      lines.push(`\nPower: ${STATE[s.power.state] || s.power.state}${s.power.milliwatts != null ? `, ${watts(s.power.milliwatts)}` : ""} (${s.power.source || ""})`);
    }
    if (parsed.findings && parsed.findings.length) {
      lines.push("\nFindings:");
      for (const f of parsed.findings) {
        lines.push(`  [${f.severity.toUpperCase()}] ${f.title}: ${f.detail}`);
      }
    }
    lines.push("\n--- Raw Data ---\n" + JSON.stringify(parsed, null, 2));
    await navigator.clipboard.writeText(lines.join("\n"));
    b.textContent = "Copied";
  } catch {
    b.textContent = "Couldn't copy";
  }
  setTimeout(() => (b.textContent = "Copy report"), 1800);
}

let charging = false;
async function chargeTest() {
  const out = $("charge-out");
  const button = $("charge-run");
  if (charging) {
    charging = false;
    return;
  }
  charging = true;
  button.textContent = "Stop";
  const samples = [];
  for (let i = 0; i < 30 && charging; i++) {
    try {
      const mw = await call("power_sample");
      if (mw != null) samples.push(mw / 1000);
      out.textContent = mw != null ? `${i + 1} of 30: ${(mw / 1000).toFixed(1)} W` : `${i + 1} of 30: no reading`;
    } catch (e) {
      out.textContent = `Test stopped: ${e}`;
      break;
    }
    await new Promise((done) => setTimeout(done, 1000));
  }
  charging = false;
  button.textContent = "Start charge test";
  if (samples.length) {
    const t = await call("charge_result", { watts: samples });
    out.textContent = `${t.summary} Average ${t.avg_w.toFixed(1)} W, lowest ${t.min_w.toFixed(1)} W, highest ${t.max_w.toFixed(1)} W.`;
  } else if (!out.textContent.startsWith("Test stopped")) {
    out.textContent = "This device gave no power readings, so the test could not run.";
  }
}

async function writeTest() {
  const out = $("write-out");
  const button = $("write-run");
  const dir = tauri ? await tauri.dialog.open({ directory: true, title: "Pick a folder on the drive to test" }) : "/demo";
  if (!dir) return;
  button.disabled = true;
  out.textContent = "Writing 256 MB…";
  try {
    const t = await call("write_test", { dir, megabytes: 256 });
    out.textContent = t.summary;
  } catch (e) {
    out.textContent = String(e);
  } finally {
    button.disabled = false;
  }
}

async function runSimulation() {
  const profile = $("sim-profile").value;
  const target = $("sim-target").value;
  const out = $("sim-out");
  const btn = $("sim-run");
  btn.disabled = true;
  btn.textContent = "Simulating…";
  try {
    const res = await call("virtual_test", { profile, target });
    out.hidden = false;
    const powerBottleneck = res.cable_is_power_bottleneck ? "Bottleneck: Cable limits charging" : "Power passed";
    const dataBottleneck = res.cable_is_data_bottleneck ? "Bottleneck: Cable limits data speed" : "Data speed maxed";
    const resRating = res.estimated_loop_resistance.rating.toLowerCase();
    const stabRating = res.stability_verdict.toLowerCase();

    out.replaceChildren(
      el("div", { class: "sim-badge-row" },
        el("span", { class: `badge ${res.cable_is_power_bottleneck ? "warn" : "ok"}` }, powerBottleneck),
        el("span", { class: `badge ${res.cable_is_data_bottleneck ? "warn" : "ok"}` }, dataBottleneck),
        el("span", { class: `badge ${res.thermal_risk ? "problem" : "ok"}` }, `Thermal: ${res.thermal_dissipation_watts.toFixed(2)} W loss`),
        el("span", { class: `badge ${stabRating.includes("solid") ? "ok" : "warn"}` }, `Stability: ${res.stability_score}% (${res.stability_verdict})`)
      ),
      grid([
        ["Negotiated Power", res.negotiated_watts ? `${res.negotiated_watts} W` : "None"],
        ["Negotiated Link", res.negotiated_mbps ? speedWords(res.negotiated_mbps) : "None"],
        ["Loop Resistance (R_loop)", `${res.estimated_loop_resistance.milliohms} mΩ (${res.estimated_loop_resistance.rating})`],
        ["Analysis Note", res.estimated_loop_resistance.note]
      ])
    );
  } catch (e) {
    out.hidden = false;
    out.replaceChildren(el("p", { class: "detail", textContent: `Simulation failed: ${e}` }));
  } finally {
    btn.disabled = false;
    btn.textContent = "Simulate cable";
  }
}

async function runQaAudit() {
  const out = $("qa-out");
  const btn = $("qa-run");
  btn.disabled = true;
  btn.textContent = "Checking…";
  try {
    const sc = await call("self_check");
    out.hidden = false;
    out.replaceChildren(
      el("div", { class: "sim-badge-row" },
        el("span", { class: `badge ${sc.passed ? "ok" : "problem"}` }, `${sc.passed_checks} of ${sc.total_checks} checks passed`)
      ),
      el("ul", { class: "findings", style: "margin-top: 10px;" },
        ...sc.notes.map((n) => el("li", { class: "finding good" }, icon("good"), el("div", {}, el("strong", { textContent: n }))))
      )
    );
  } catch (e) {
    out.hidden = false;
    out.replaceChildren(el("p", { class: "detail", textContent: `Self-check error: ${e}` }));
  } finally {
    btn.disabled = false;
    btn.textContent = "Run self-check";
  }
}

$("refresh").addEventListener("click", refresh);
$("copy").addEventListener("click", copyReport);
$("charge-run").addEventListener("click", chargeTest);
$("write-run").addEventListener("click", writeTest);
if ($("sim-run")) $("sim-run").addEventListener("click", runSimulation);
if ($("qa-run")) $("qa-run").addEventListener("click", runQaAudit);
refresh();
let pollTimer = setInterval(refresh, 3000);
document.addEventListener("visibilitychange", () => {
  if (document.hidden) {
    clearInterval(pollTimer);
    pollTimer = null;
  } else {
    refresh();
    if (!pollTimer) {
      pollTimer = setInterval(refresh, 3000);
    }
  }
});
