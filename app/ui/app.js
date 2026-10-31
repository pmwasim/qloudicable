// QloudiCable front end. The Rust core decides every result; this file shows it.
const $ = (id) => document.getElementById(id);

function el(tag, props = {}, ...kids) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") node.className = v;
    else if (k === "role" || k.startsWith("aria-")) node.setAttribute(k, v);
    else node[k] = v;
  }
  for (const kid of kids) if (kid !== null && kid !== undefined) node.append(kid);
  return node;
}

const SVG_NS = "http://www.w3.org/2000/svg";
const svgEl = (tag, attrs = {}, ...kids) => {
  const n = document.createElementNS(SVG_NS, tag);
  for (const [k, v] of Object.entries(attrs)) n.setAttribute(k, v);
  for (const kid of kids) if (kid) n.append(kid);
  return n;
};
const svgText = (attrs, text) => {
  const t = svgEl("text", attrs);
  t.textContent = text;
  return t;
};

const tauri = window.__TAURI__;
const call = (cmd, args) =>
  tauri ? tauri.core.invoke(cmd, args) : Promise.reject(new Error("Open the QloudiCable app to use this."));

const reduceMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;
const speedWords = (m) => (m === 2 ? "1.5 Mbps" : m >= 1000 ? `${m / 1000} Gbps` : `${m} Mbps`);
const watts = (mw) => `${(mw / 1000).toFixed(mw % 1000 ? 1 : 0)} W`;
const hex = (n, w) => n.toString(16).padStart(w, "0");

const CABLE_SPEED = {
  usb2: "USB 2.0 speed",
  usb5g: "5 Gbps",
  usb20g: "10 to 20 Gbps",
  usb40g: "40 Gbps",
  usb80g: "80 Gbps",
  reserved: "speed not stated",
};
const CABLE_MBPS = { usb2: 480, usb5g: 5000, usb20g: 20000, usb40g: 40000, usb80g: 80000 };
const TRANSPORT = { USB2: "USB 2.0", USB3: "USB 3", USB: "USB", DP: "DisplayPort", DisplayPort: "DisplayPort", TB3: "Thunderbolt 3", TB4: "Thunderbolt 4", Thunderbolt: "Thunderbolt" };
const CHARGE_STATE = { charging: "Charging", discharging: "On battery", full: "Battery full", not_charging: "Plugged in, not charging", unknown: "Unknown" };
const SEVERITY_WORD = { good: "Good", info: "Note", warn: "Check", problem: "Problem" };
const SEVERITY_GLYPH = { good: "✓", info: "i", warn: "!", problem: "✕" };
const ACCESS_WORD = { full: "Yes", partial: "Partly", none: "No" };
const ACTION_LABEL = { guide: "Run the both-ends test", watch: "Start a port watch", charge: "Start a charging test" };
const SHEETS = { charge: "panel-charge", watch: "panel-watch", guide: "panel-guide", drive: "panel-drive", details: "sheet-details", tools: "sheet-tools" };

let report = null;
let lastJson = null;
let busy = false;
let showAllDevices = false;
const powerHistory = [];
let lastTrigger = null;

// ---- The answer

function heroFor(r) {
  const s = r.snapshot;
  const worst = r.findings[0];
  const hasDevices = s.usb_devices.some((d) => !d.is_hub);
  const connected = s.ports.some((p) => p.connected);
  const blind = s.capabilities.cable_identity.level === "none" && !s.power;

  if (worst && (worst.severity === "problem" || worst.severity === "warn")) {
    const cableRelated = worst.code.startsWith("cable") || worst.code === "unusual_emarker";
    return { level: worst.severity, status: SEVERITY_WORD[worst.severity], title: worst.title, text: worst.detail, action: cableRelated ? "guide" : "watch" };
  }
  if (worst && worst.severity === "good") return { level: "good", status: "All clear", title: "No problems found", text: worst.detail, action: null };
  if (connected || hasDevices) return { level: "good", status: "All clear", title: "No problems found", text: "Everything this device reports looks normal.", action: null };
  if (blind) return { level: "info", status: "Limited view", title: "This device can't see cable details", text: "It still shows devices and power. Run a test to check a cable yourself.", action: "charge" };
  return { level: "idle", status: "Waiting", title: "Plug in a cable to start", text: "Connect a charger or device. This screen updates by itself.", action: "watch" };
}

// True when the hero already states the most important finding.
function heroCovers(r) {
  const w = r.findings[0];
  return !!w && (w.severity === "warn" || w.severity === "problem");
}

function renderStage(r) {
  const h = heroFor(r);
  const stage = $("stage");
  stage.dataset.level = h.level;
  $("hero-status").textContent = h.status;
  $("hero-title").textContent = h.title;
  $("hero-text").textContent = h.text;
  const btn = $("hero-action");
  btn.hidden = !h.action;
  if (h.action) {
    btn.textContent = ACTION_LABEL[h.action];
    btn.onclick = () => openSheet(h.action);
  }
  // The most important findings, as tappable chips. The rest live in the Findings sheet.
  // The hero already states the most important finding, so chips start after it.
  const rest = heroCovers(r) ? r.findings.slice(1) : r.findings;
  const chips = rest.slice(0, 3).map((f) =>
    el("button", { type: "button", class: "finding-chip", textContent: f.title, onclick: (e) => openSheet("details", e.currentTarget) })
  );
  if (rest.length > 3) {
    chips.push(el("button", { type: "button", class: "finding-chip", textContent: `+${rest.length - 3} more`, onclick: (e) => openSheet("details", e.currentTarget) }));
  }
  $("findings-chips").replaceChildren(...chips);
  renderCable(r);
}

// ---- The cable diagram: the one picture that carries the answer

function renderCable(r) {
  const s = r.snapshot;
  const svg = $("stage-cable");
  const port = s.ports.find((p) => p.connected);
  const device = s.usb_devices.filter((d) => !d.is_hub).sort((a, b) => (b.speed_mbps || 0) - (a.speed_mbps || 0))[0];
  const linkMbps = device?.speed_mbps || (port?.cable ? CABLE_MBPS[port.cable.speed] : null);
  const mw = port?.active_pdo?.max_mw || s.power?.milliwatts || null;
  const fast = linkMbps && linkMbps >= 5000;

  svg.replaceChildren();
  svg.setAttribute("viewBox", "0 0 680 150");
  svg.setAttribute("aria-label", port ? `Cable from this computer to ${port.partner?.vendor_name || port.partner?.kind || "a device"}` : "No cable connected");

  svg.append(
    svgEl("rect", { x: 20, y: 40, width: 150, height: 70, rx: 16, class: "cable-box" }),
    svgText({ x: 95, y: 82, "text-anchor": "middle", class: "cable-label" }, "This computer"),
    svgEl("rect", { x: 510, y: 40, width: 150, height: 70, rx: 16, class: "cable-box" }),
    svgText({ x: 585, y: 78, "text-anchor": "middle", class: "cable-label" }, port ? (port.partner?.vendor_name || port.partner?.kind || "Device or charger") : "Nothing connected"),
    svgText({ x: 585, y: 98, "text-anchor": "middle", class: "cable-sub" }, port ? "" : "Plug something in"),
    svgEl("path", { d: "M170 75 L510 75", class: "cable-body", ...(port ? {} : { "stroke-dasharray": "4 10" }) })
  );
  if (port) {
    svg.append(
      svgEl("path", { d: "M170 75 L510 75", class: fast ? "cable-flow" : "cable-flow", style: `--flow-speed: ${fast ? "0.6s" : "1.6s"}` }),
      svgEl("rect", { x: 166, y: 61, width: 18, height: 28, rx: 5, class: "cable-plug" }),
      svgEl("rect", { x: 496, y: 61, width: 18, height: 28, rx: 5, class: "cable-plug" }),
      svgText({ x: 340, y: 56, "text-anchor": "middle", class: "cable-label" }, linkMbps ? speedWords(linkMbps) : "Speed not reported"),
      svgText({ x: 340, y: 110, "text-anchor": "middle", class: "cable-sub" }, mw ? `${watts(mw)} available` : port.cable ? (port.cable.max_watts ? `Rated ${port.cable.max_watts} W` : "Power not stated") : "No cable detail")
    );
    if (port.cable) svg.append(svgEl("rect", { x: 300, y: 69, width: 16, height: 12, rx: 3, class: "cable-chip" }));
  }
}

// ---- Ports, devices, power, capabilities

function portCard(p, identity) {
  const card = el("article", { class: p.connected ? "port connected" : "port" });
  card.append(
    el("div", { class: "port-head" },
      el("span", { class: "port-name" }, el("span", { class: "dot", "aria-hidden": "true" }), p.label),
      el("span", { class: "chip", textContent: p.connected ? "Connected" : "Empty" })
    )
  );
  if (!p.connected) {
    card.append(el("p", { class: "port-line", textContent: "Nothing plugged in" }));
    return card;
  }
  const line = p.cable
    ? `${CABLE_SPEED[p.cable.speed]} · ${p.cable.max_watts ? p.cable.max_watts + " W" : "power not stated"}`
    : identity === "full" ? "No e-marker: limited to 60 W" : "Cable detail not shown";
  card.append(el("p", { class: "port-line", textContent: line }));

  const details = el("details", { class: "more" }, el("summary", { textContent: "Details" }));
  const dl = el("dl", {});
  const add = (k, v) => v && dl.append(el("dt", { textContent: k }), el("dd", { textContent: v }));
  if (p.cable) add("Maker", p.cable.vendor_name || `Unknown (${hex(p.cable.vendor_id, 4)})`);
  if (p.partner && (p.partner.kind || p.partner.vendor_name)) add("Connected to", [p.partner.vendor_name, p.partner.kind].filter(Boolean).join(", "));
  const links = p.transports.filter((t) => t !== "CC").map((t) => TRANSPORT[t] || t);
  if (links.length) add("Data", links.join(", "));
  add("Data direction", p.data_role ? ({ host: "Host (this computer)", device: "Device" }[p.data_role] || p.data_role) : null);
  add("Power direction", p.power_role ? ({ source: "Gives power", sink: "Takes power" }[p.power_role] || p.power_role) : null);
  details.append(dl);
  if (p.source_pdos.length) {
    const active = p.active_pdo && p.active_pdo.max_mw;
    details.append(
      el("p", { textContent: "Charger offers" }),
      el("div", { class: "pdo-list" }, ...p.source_pdos.map((o) => {
        const volts = o.min_mv === o.max_mv ? `${o.max_mv / 1000} V` : `${o.min_mv / 1000} to ${o.max_mv / 1000} V`;
        return el("span", { class: o.max_mv === active ? "pdo-chip active" : "pdo-chip", textContent: `${volts}, ${watts(o.max_mw)}` });
      }))
    );
  }
  if (p.cable) details.append(el("code", { textContent: p.cable.raw_vdos.map((v) => "0x" + hex(v, 8)).join(" ") }));
  card.append(details);
  return card;
}

function renderPorts(s) {
  const identity = s.capabilities.cable_identity.level;
  $("ports").replaceChildren(
    ...(s.ports.length
      ? s.ports.map((p) => portCard(p, identity))
      : [el("article", { class: "port" }, el("p", { class: "port-line", textContent: s.capabilities.cable_identity.why || "No USB-C ports are reported here." }))])
  );
}

function renderDevices(s) {
  const list = s.usb_devices.filter((d) => !d.is_hub);
  const shown = showAllDevices ? list : list.slice(0, 4);
  $("devices").replaceChildren(
    ...(list.length
      ? shown.map((d) => {
          const name = d.product_name || d.vendor_name || "Unnamed device";
          const sub = d.usb_version ? `USB ${d.usb_version}` : `${hex(d.vendor_id, 4)}:${hex(d.product_id, 4)}`;
          return el("div", { class: "device" },
            el("div", {}, name, el("span", { class: "sub", textContent: sub })),
            el("span", { class: "val", textContent: d.speed_mbps ? speedWords(d.speed_mbps) : "Speed not reported" }));
        })
      : [el("p", { class: "port-line", textContent: "No devices connected." })])
  );
  const more = $("devices-more");
  more.hidden = list.length <= 4;
  more.textContent = showAllDevices ? "Show fewer" : `Show all ${list.length}`;
}

function drawSpark() {
  const svg = $("spark");
  svg.replaceChildren();
  const W = 300, H = 70, values = powerHistory.slice(-40);
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  if (values.length < 2) return;
  const max = Math.max(10, ...values) * 1.15;
  const x = (i) => 8 + (i / (values.length - 1)) * (W - 16);
  const y = (v) => H - 8 - (v / max) * (H - 16);
  const d = values.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
  svg.append(
    svgEl("path", { d: `${d} L${x(values.length - 1)},${H} L${x(0)},${H} Z`, class: "fill" }),
    svgEl("path", { d, class: "line" })
  );
}

function renderPower(s) {
  const p = s.power;
  const ring = $("power-ring");
  $("power-big").textContent = p && p.milliwatts != null ? watts(p.milliwatts) : "No reading";
  $("power-state").textContent = p ? CHARGE_STATE[p.state] || CHARGE_STATE.unknown : "No battery here";
  if (p && p.battery_percent != null) window.QCViz.ring(ring, p.battery_percent, "Battery");
  else ring.replaceChildren();
  $("power-stats").replaceChildren(
    ...[
      ["Battery", p && p.battery_percent != null ? `${p.battery_percent}%` : "—"],
      ["Volts", p && p.voltage_mv ? `${(p.voltage_mv / 1000).toFixed(1)} V` : "—"],
      ["Amps", p && p.current_ma ? `${(p.current_ma / 1000).toFixed(2)} A` : "—"],
    ].map(([label, value]) => el("div", { class: "tile" }, el("strong", { textContent: value }), el("span", { textContent: label })))
  );
}

function renderAccess(s) {
  const c = s.capabilities;
  const rows = [["Cable chip", c.cable_identity], ["Charger profiles", c.charger_profiles], ["Link speed", c.link_speed], ["Power reading", c.power_measurement]];
  $("access").replaceChildren(
    ...rows.map(([name, a]) =>
      el("div", { class: "access-row", title: a.why || "" },
        el("span", { textContent: name }),
        el("span", { class: a.level === "full" ? "chip on" : "chip", textContent: ACCESS_WORD[a.level] })))
  );
  $("version").textContent = s.tool_version;
  // Drive speed is a desktop test. Hide its dock button on phones.
  const android = s.platform === "android";
  $("tab-drive").hidden = android;
  if (android && !$(SHEETS.drive).hidden) closeSheet();
}

function renderFindingsList(r) {
  $("findings").replaceChildren(
    ...r.findings.map((f) =>
      el("li", { class: `finding ${f.severity}` },
        el("span", { class: "glyph", "aria-hidden": "true", textContent: SEVERITY_GLYPH[f.severity] }),
        el("div", {},
          el("p", { class: "f-title" }, el("span", { class: "sr-only", textContent: `${SEVERITY_WORD[f.severity]}: ` }), f.title),
          el("p", { class: "f-detail", textContent: f.detail })))
    )
  );
}

// ---- Sheets: tests and tools open over the screen, never below it

function openSheet(name, trigger) {
  const id = SHEETS[name];
  if (!id) return;
  lastTrigger = trigger || document.activeElement;
  for (const other of Object.values(SHEETS)) if (other !== id) $(other).hidden = true;
  $(id).hidden = false;
  $("scrim").hidden = false;
  for (const key of ["charge", "watch", "guide", "drive"]) {
    const tab = $(`tab-${key}`);
    tab.setAttribute("aria-expanded", key === name ? "true" : "false");
  }
  const first = $(id).querySelector("button, [href], summary") || $(id);
  first.focus();
}

function closeSheet() {
  for (const id of Object.values(SHEETS)) $(id).hidden = true;
  $("scrim").hidden = true;
  if (lastTrigger && lastTrigger.focus) lastTrigger.focus();
}

function visibleDock() {
  return ["tab-charge", "tab-watch", "tab-guide", "tab-drive", "open-details", "open-tools"].map((id) => $(id)).filter((b) => !b.hidden);
}

function onDockKey(e) {
  const order = visibleDock();
  const i = order.indexOf(e.currentTarget);
  let next = null;
  if (e.key === "ArrowRight") next = order[(i + 1) % order.length];
  if (e.key === "ArrowLeft") next = order[(i - 1 + order.length) % order.length];
  if (e.key === "Home") next = order[0];
  if (e.key === "End") next = order[order.length - 1];
  if (next) {
    e.preventDefault();
    next.focus();
  }
}

// ---- Tests

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
      if (window.QCFeatures) window.QCFeatures.live(samples);
    } catch (e) {
      out.textContent = `Test stopped: ${e}`;
      break;
    }
    await new Promise((done) => setTimeout(done, 1000));
  }
  charging = false;
  button.textContent = "Start charging test";
  if (samples.length) {
    const t = await call("charge_result", { watts: samples });
    out.textContent = `${t.summary} Average ${t.avg_w.toFixed(1)} W, lowest ${t.min_w.toFixed(1)} W, highest ${t.max_w.toFixed(1)} W.`;
    if (window.QCFeatures) window.QCFeatures.charged(samples, t);
  } else if (!out.textContent.startsWith("Test stopped")) {
    out.textContent = "No power readings came back, so the test could not run.";
  }
}

async function writeTest() {
  const out = $("write-out");
  const button = $("write-run");
  const dir = await tauri.dialog.open({ directory: true, title: "Pick a folder on the drive to test" });
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

async function runSelfCheck() {
  const out = $("qa-out");
  const btn = $("qa-run");
  btn.disabled = true;
  btn.textContent = "Checking…";
  try {
    const sc = await call("self_check");
    out.hidden = false;
    out.replaceChildren(
      el("p", { class: "out", textContent: `${sc.passed_checks} of ${sc.total_checks} checks passed` }),
      el("ul", { class: "findings" }, ...sc.audit_notes.map((n) => el("li", { class: "f-detail", textContent: n })))
    );
  } catch (e) {
    out.hidden = false;
    out.replaceChildren(el("p", { class: "out", textContent: `Self-check error: ${e}` }));
  } finally {
    btn.disabled = false;
    btn.textContent = "Run self-check";
  }
}

// ---- Copy

function summaryText(r) {
  const s = r.snapshot;
  const h = heroFor(r);
  const lines = [`QloudiCable on ${s.platform}: ${h.title}.`];
  for (const p of s.ports.filter((x) => x.connected)) {
    const cable = p.cable ? `${CABLE_SPEED[p.cable.speed]}, ${p.cable.max_watts ? p.cable.max_watts + " W" : "power not stated"}` : "no cable detail";
    lines.push(`${p.label}: ${cable}`);
  }
  for (const f of r.findings.slice(0, 3)) lines.push(`${SEVERITY_WORD[f.severity]}: ${f.title}`);
  lines.push("Checked with QloudiCable. Nothing was sent anywhere.");
  return lines.join("\n");
}

async function copyText(text, button, doneLabel) {
  const label = button.textContent;
  try {
    await navigator.clipboard.writeText(text);
    button.textContent = doneLabel;
  } catch {
    button.textContent = "Couldn't copy";
  }
  setTimeout(() => (button.textContent = label), 1800);
}

// ---- Refresh

function render(r) {
  report = r;
  const s = r.snapshot;
  renderStage(r);
  renderPorts(s);
  renderDevices(s);
  if (s.power && s.power.milliwatts != null) powerHistory.push(s.power.milliwatts / 1000);
  renderPower(s);
  drawSpark();
  renderAccess(s);
  renderFindingsList(r);
}

async function refresh() {
  if (busy) return;
  busy = true;
  try {
    const r = await call("report");
    $("error").hidden = true;
    $("updated").textContent = `Checked ${new Date().toLocaleTimeString()}`;
    const text = JSON.stringify(r);
    if (text !== lastJson) {
      lastJson = text;
      render(r);
    }
  } catch (e) {
    $("error").textContent = `Couldn't read this device: ${e}`;
    $("error").hidden = false;
  } finally {
    busy = false;
  }
}

// ---- Wiring

$("refresh").addEventListener("click", refresh);
$("copy").addEventListener("click", () => report && copyText(summaryText(report), $("copy"), "Copied"));
$("copy-json").addEventListener("click", () => report && copyText(JSON.stringify(report, null, 2), $("copy-json"), "Copied"));
$("devices-more").addEventListener("click", () => {
  showAllDevices = !showAllDevices;
  if (report) renderDevices(report.snapshot);
});
$("charge-run").addEventListener("click", chargeTest);
$("write-run").addEventListener("click", writeTest);
$("qa-run").addEventListener("click", runSelfCheck);
$("scrim").addEventListener("click", closeSheet);
$("open-details").addEventListener("click", (e) => openSheet("details", e.currentTarget));
$("open-tools").addEventListener("click", (e) => openSheet("tools", e.currentTarget));
document.querySelectorAll("[data-close]").forEach((b) => b.addEventListener("click", closeSheet));
for (const key of ["charge", "watch", "guide", "drive"]) {
  const btn = $(`tab-${key}`);
  btn.addEventListener("click", (e) => openSheet(key, e.currentTarget));
  btn.addEventListener("keydown", onDockKey);
}
["open-details", "open-tools"].forEach((id) => $(id).addEventListener("keydown", onDockKey));
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !$("scrim").hidden) closeSheet();
});

refresh();
let pollTimer = setInterval(refresh, 3000);
document.addEventListener("visibilitychange", () => {
  if (document.hidden) {
    clearInterval(pollTimer);
  } else {
    refresh();
    pollTimer = setInterval(refresh, 3000);
  }
});
