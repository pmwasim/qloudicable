// QloudiCable dashboard. Three parts:
//   Store      holds the latest report, power history and the selected tile.
//   Components each draw one region from the store and redraw when it changes.
//   Wiring    connects buttons and keys to store updates.
// The Rust core decides every result. This file only shows it.

// ---- Helpers ------------------------------------------------------------

const $ = (id) => document.getElementById(id);

function el(tag, props = {}, ...kids) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (v === undefined || v === null) continue;
    if (k === "class") node.className = v;
    else if (k === "role" || k.startsWith("aria-") || k.startsWith("data-")) node.setAttribute(k, v);
    else if (k.startsWith("on")) node.addEventListener(k.slice(2), v);
    else node[k] = v;
  }
  for (const kid of kids) if (kid !== null && kid !== undefined && kid !== false) node.append(kid);
  return node;
}

const SVG_NS = "http://www.w3.org/2000/svg";
function svgEl(tag, attrs = {}, ...kids) {
  const n = document.createElementNS(SVG_NS, tag);
  for (const [k, v] of Object.entries(attrs)) if (v !== undefined && v !== null) n.setAttribute(k, v);
  for (const kid of kids) if (kid) n.append(kid);
  return n;
}
function svgText(attrs, text) {
  const t = svgEl("text", attrs);
  t.textContent = text;
  return t;
}

const tauri = window.__TAURI__;
const call = (cmd, args) =>
  tauri ? tauri.core.invoke(cmd, args) : Promise.reject(new Error("Open the QloudiCable app to use this."));
const reduceMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

const speedWords = (m) => (m === 2 ? "1.5 Mbps" : m >= 1000 ? `${m / 1000} Gbps` : `${m} Mbps`);
const watts = (mw) => `${(mw / 1000).toFixed(mw % 1000 ? 1 : 0)} W`;
const hex = (n, w) => n.toString(16).padStart(w, "0");

const CABLE_SPEED = { usb2: "USB 2.0 speed", usb5g: "5 Gbps", usb20g: "10 to 20 Gbps", usb40g: "40 Gbps", usb80g: "80 Gbps", reserved: "speed not stated" };
const CABLE_MBPS = { usb2: 480, usb5g: 5000, usb20g: 20000, usb40g: 40000, usb80g: 80000 };
const TRANSPORT = { USB2: "USB 2.0", USB3: "USB 3", USB: "USB", DP: "DisplayPort", DisplayPort: "DisplayPort", TB3: "Thunderbolt 3", TB4: "Thunderbolt 4", Thunderbolt: "Thunderbolt" };
const CHARGE_STATE = { charging: "Charging", discharging: "On battery", full: "Battery full", not_charging: "Plugged in, not charging", unknown: "Unknown" };
const SEVERITY_WORD = { good: "Good", info: "Note", warn: "Check", problem: "Problem" };
const SEVERITY_GLYPH = { good: "✓", info: "i", warn: "!", problem: "✕" };
const ACCESS_WORD = { full: "Yes", partial: "Partly", none: "No" };
const ACTION_LABEL = { guide: "Run the both-ends test", watch: "Start a port watch", charge: "Start a charging test" };
const SHEETS = { charge: "panel-charge", watch: "panel-watch", guide: "panel-guide", drive: "panel-drive", details: "sheet-details", tools: "sheet-tools", tests: "sheet-tests" };
const POWER_HISTORY_MAX = 60;

// ---- Store ----------------------------------------------------------------

const Store = (() => {
  let state = { report: null, lastJson: null, history: [], selected: null, busy: false, showAllDevices: false };
  const subs = new Set();
  return {
    get: () => state,
    set(patch) {
      state = { ...state, ...patch };
      for (const f of subs) f(state);
    },
    subscribe(f) {
      subs.add(f);
      return () => subs.delete(f);
    },
    pushPower(watts) {
      const history = [...state.history, watts].slice(-POWER_HISTORY_MAX);
      state = { ...state, history };
    },
  };
})();

// ---- Derived facts (presentation only) -----------------------------------

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

function activePort(s) {
  return s.ports.find((p) => p.connected) || null;
}

function linkMbps(s) {
  const port = activePort(s);
  const device = s.usb_devices.filter((d) => !d.is_hub).sort((a, b) => (b.speed_mbps || 0) - (a.speed_mbps || 0))[0];
  return device?.speed_mbps || (port?.cable ? CABLE_MBPS[port.cable.speed] : null) || null;
}

function powerMw(s) {
  const port = activePort(s);
  return port?.active_pdo?.max_mw || s.power?.milliwatts || null;
}

// Findings are grouped by the tile they concern. The core supplies the codes.
const TILE_CODES = {
  power: ["measured_charge_rate", "negotiated_below_limit", "charging_at_limit", "plugged_not_charging", "battery_full"],
  cable: ["cable_limits_charging", "unusual_emarker"],
  link: ["cable_limits_data", "link_below_device", "link_at_device_max"],
  battery: ["battery_full", "plugged_not_charging"],
  ports: [],
  devices: [],
};
function findingsFor(r, tile) {
  const codes = TILE_CODES[tile] || [];
  return r.findings.filter((f) => codes.includes(f.code));
}

// ---- Tile model -------------------------------------------------------------

function tileModels(r) {
  const s = r.snapshot;
  const port = activePort(s);
  const p = s.power;
  const link = linkMbps(s);
  const mw = powerMw(s);
  const cableLimited = r.findings.some((f) => f.code === "cable_limits_charging" || f.code === "cable_limits_data");
  const connectedCount = s.ports.filter((x) => x.connected).length;
  const devices = s.usb_devices.filter((d) => !d.is_hub);

  return [
    {
      id: "power",
      title: "Power",
      value: p?.milliwatts != null ? watts(p.milliwatts) : mw ? watts(mw) : "—",
      sub: p ? CHARGE_STATE[p.state] || CHARGE_STATE.unknown : "No battery here",
      level: cableLimited ? "warn" : "neutral",
      spark: true,
    },
    {
      id: "battery",
      title: "Battery",
      value: p && p.battery_percent != null ? `${p.battery_percent}%` : "—",
      sub: p && p.battery_percent != null ? "Read from the battery" : "No battery reading",
      bar: p && p.battery_percent != null ? p.battery_percent : null,
      level: "neutral",
    },
    {
      id: "cable",
      title: "Cable",
      value: port?.cable ? (port.cable.max_watts ? `${port.cable.max_watts} W` : "Power not stated") : port ? "No e-marker" : "—",
      sub: port?.cable ? CABLE_SPEED[port.cable.speed] : port ? "Limited to 60 W" : "Nothing connected",
      chip: port?.cable ? "e-marker" : null,
      level: cableLimited ? "warn" : port?.cable ? "good" : "neutral",
    },
    {
      id: "link",
      title: "Link",
      value: link ? speedWords(link) : "—",
      sub: devices[0] ? (devices[0].product_name || devices[0].vendor_name || "Device") : "No device",
      level: r.findings.some((f) => f.code === "link_below_device" || f.code === "cable_limits_data") ? "warn" : "neutral",
    },
    {
      id: "ports",
      title: "Ports",
      value: `${connectedCount} of ${s.ports.length}`,
      sub: connectedCount ? "Connected" : "Nothing plugged in",
      level: "neutral",
    },
    {
      id: "devices",
      title: "Devices",
      value: String(devices.length),
      sub: devices.length ? "Connected now" : "None connected",
      level: "neutral",
    },
  ];
}

// ---- Components -----------------------------------------------------------

// Verdict: the one answer, the one action, and the power path.
const Verdict = {
  mount() {},
  update(state) {
    const r = state.report;
    if (!r) return;
    const h = heroFor(r);
    $("verdict").dataset.level = h.level;
    $("hero-status").textContent = h.status;
    $("hero-title").textContent = h.title;
    $("hero-text").textContent = h.text;
    const btn = $("hero-action");
    btn.hidden = !h.action;
    if (h.action) {
      btn.textContent = ACTION_LABEL[h.action];
      btn.onclick = (e) => openSheet(h.action, e.currentTarget);
    }
    const more = $("open-details");
    more.hidden = r.findings.length === 0;
    more.textContent = `Findings (${r.findings.length})`;
    more.onclick = (e) => openSheet("details", e.currentTarget);
    $("findings").replaceChildren(...findingItems(r.findings));
    drawPath(r);
  },
};

function drawPath(r) {
  const s = r.snapshot;
  const port = activePort(s);
  const svg = $("path");
  const link = linkMbps(s);
  const mw = powerMw(s);
  const fast = link && link >= 5000;
  svg.replaceChildren();
  svg.setAttribute("viewBox", "0 0 640 120");
  svg.setAttribute("aria-label", port ? `Path from the charger to this computer: ${port.label}` : "Nothing connected");
  svg.append(
    svgEl("rect", { x: 8, y: 30, width: 150, height: 60, rx: 14, class: "path-box" }),
    svgText({ x: 83, y: 66, "text-anchor": "middle", class: "path-label" }, "Charger"),
    svgEl("rect", { x: 482, y: 30, width: 150, height: 60, rx: 14, class: "path-box" }),
    svgText({ x: 557, y: 66, "text-anchor": "middle", class: "path-label" }, "This computer"),
    svgEl("path", { d: "M158 60 L482 60", class: port ? "path-wire" : "path-wire idle" })
  );
  if (port) {
    svg.append(svgEl("path", { d: "M158 60 L482 60", class: "path-flow", style: `--flow-speed: ${fast ? "0.6s" : "1.6s"}` }));
    svg.append(svgText({ x: 320, y: 44, "text-anchor": "middle", class: "path-sub" }, link ? speedWords(link) : "Speed not reported"));
    svg.append(svgText({ x: 320, y: 90, "text-anchor": "middle", class: "path-sub" }, mw ? `${watts(mw)} available` : port.cable?.max_watts ? `Cable rated ${port.cable.max_watts} W` : "Power not stated"));
  } else {
    svg.append(svgText({ x: 320, y: 90, "text-anchor": "middle", class: "path-sub" }, "Nothing connected"));
  }
}

// Tiles: the live readings. A tile is a button, so it works with keyboard and screen readers.
const Tiles = {
  mount() {},
  update(state) {
    const r = state.report;
    if (!r) return;
    const models = tileModels(r);
    const host = $("tiles");
    // Reuse existing tile nodes so values can animate between updates.
    const existing = new Map([...host.children].map((n) => [n.dataset.id, n]));
    const nodes = models.map((m) => {
      let node = existing.get(m.id);
      if (!node) {
        node = el("button", { type: "button", class: "tile", "data-id": m.id, "aria-controls": "detail" },
          el("span", { class: "tile-title", textContent: m.title }),
          el("strong", { class: "tile-value" }),
          el("span", { class: "tile-sub" }),
          el("span", { class: "tile-extra" }),
          el("span", { class: "tile-bar" }, el("i")));
        node.addEventListener("click", () => selectTile(m.id));
      }
      node.dataset.level = m.level;
      node.setAttribute("aria-pressed", state.selected === m.id ? "true" : "false");
      node.classList.toggle("selected", state.selected === m.id);
      node.querySelector(".tile-value").textContent = m.value;
      node.querySelector(".tile-sub").textContent = m.sub;
      const extra = node.querySelector(".tile-extra");
      extra.replaceChildren(...(m.chip ? [el("span", { class: "chip", textContent: m.chip })] : []));
      if (m.bar != null) {
        node.querySelector(".tile-bar").hidden = false;
        node.querySelector(".tile-bar i").style.width = `${m.bar}%`;
      } else {
        node.querySelector(".tile-bar").hidden = true;
      }
      if (m.spark) {
        if (!node.querySelector(".tile-spark")) node.append(svgEl("svg", { class: "tile-spark", viewBox: "0 0 120 30", "aria-hidden": "true" }));
        drawSpark(node.querySelector(".tile-spark"), state.history, 120, 30);
      } else {
        node.querySelector(".tile-spark")?.remove();
      }
      return node;
    });
    host.replaceChildren(...nodes);
  },
};

function drawSpark(svg, values, W, H) {
  svg.replaceChildren();
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  if (values.length < 2) return;
  const max = Math.max(10, ...values) * 1.15;
  const x = (i) => 3 + (i / (values.length - 1)) * (W - 6);
  const y = (v) => H - 3 - (v / max) * (H - 6);
  const d = values.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
  svg.append(svgEl("path", { d: `${d} L${x(values.length - 1)},${H} L${x(0)},${H} Z`, class: "spark-fill" }), svgEl("path", { d, class: "spark-line" }));
}

// Detail: everything about the selected tile, in plain words, with its findings.
const Detail = {
  mount() {},
  update(state) {
    const r = state.report;
    const dash = $("dash");
    const open = !!state.selected;
    dash.dataset.detail = open ? "open" : "closed";
    if (!r || !open) {
      $("detail-title").textContent = "Pick a reading";
      $("detail-body").replaceChildren(el("p", { class: "muted", textContent: "Tap Power, Battery, Cable, Link, Ports or Devices to see how it was measured." }));
      return;
    }
    const model = tileModels(r).find((m) => m.id === state.selected);
    $("detail-title").textContent = model.title;
    $("detail-body").replaceChildren(...detailFor(state.selected, r, state));
  },
};

function row(label, value) {
  return value ? el("div", { class: "kv" }, el("span", { textContent: label }), el("strong", { textContent: value })) : null;
}

function findingList(r, tile) {
  const list = findingsFor(r, tile);
  if (!list.length) return null;
  return el("ul", { class: "findings" }, ...findingItems(list));
}

function findingItems(list) {
  return list.map((f) =>
    el("li", { class: `finding ${f.severity}` },
      el("span", { class: "glyph", "aria-hidden": "true", textContent: SEVERITY_GLYPH[f.severity] }),
      el("div", {},
        el("p", { class: "f-title" }, el("span", { class: "sr-only", textContent: `${SEVERITY_WORD[f.severity]}: ` }), f.title),
        el("p", { class: "f-detail", textContent: f.detail }))));
}

function detailFor(id, r, state) {
  const s = r.snapshot;
  const port = activePort(s);
  const p = s.power;
  const parts = [];

  if (id === "power") {
    const svg = svgEl("svg", { class: "detail-spark", viewBox: "0 0 300 90", role: "img", "aria-label": "Power over the last minute" });
    drawSpark(svg, state.history, 300, 90);
    parts.push(svg, el("div", { class: "kv-list" },
      row("State", p ? CHARGE_STATE[p.state] : null),
      row("Voltage", p?.voltage_mv ? `${(p.voltage_mv / 1000).toFixed(2)} V` : null),
      row("Current", p?.current_ma ? `${(p.current_ma / 1000).toFixed(2)} A` : null),
      row("Read from", p ? (/battery/i.test(p.source) ? "The battery" : p.source) : null)));
  }
  if (id === "battery") {
    parts.push(el("div", { class: "kv-list" }, row("Charge", p?.battery_percent != null ? `${p.battery_percent}%` : "Not reported"), row("State", p ? CHARGE_STATE[p.state] : null)));
  }
  if (id === "cable") {
    if (port?.cable) {
      const c = port.cable;
      parts.push(el("div", { class: "kv-list" },
        row("Speed", CABLE_SPEED[c.speed]),
        row("Power", c.max_watts ? `${c.max_watts} W (${c.current_a} A)` : "Not stated"),
        row("Type", c.kind),
        row("Maker", c.vendor_name || `Unknown (${hex(c.vendor_id, 4)})`),
        row("Raw data", c.raw_vdos.map((v) => "0x" + hex(v, 8)).join(" "))));
    } else {
      parts.push(el("p", { class: "muted", textContent: port ? "No e-marker reported, so this cable is limited to 60 W." : "Connect a cable to see its details." }));
    }
  }
  if (id === "link") {
    parts.push(el("div", { class: "kv-list" }, ...s.usb_devices.filter((d) => !d.is_hub).map((d) => row(d.product_name || d.vendor_name || "Device", d.speed_mbps ? speedWords(d.speed_mbps) : "Speed not reported"))));
    if (!s.usb_devices.length) parts.push(el("p", { class: "muted", textContent: "No devices connected." }));
  }
  if (id === "ports") {
    parts.push(...s.ports.map((pt) => el("div", { class: "port-row" },
      el("span", { class: pt.connected ? "dot on" : "dot", "aria-hidden": "true" }),
      el("div", {}, el("strong", { textContent: pt.label }), el("span", { class: "sub", textContent: pt.connected ? (pt.partner?.vendor_name || pt.partner?.kind || "Connected") : "Empty" })))));
    if (!s.ports.length) parts.push(el("p", { class: "muted", textContent: s.capabilities.cable_identity.why || "No USB-C ports reported." }));
  }
  if (id === "devices") {
    parts.push(...s.usb_devices.filter((d) => !d.is_hub).map((d) => el("div", { class: "kv" },
      el("span", {}, d.product_name || d.vendor_name || "Device", el("span", { class: "sub", textContent: d.usb_version ? `USB ${d.usb_version}` : "" })),
      el("strong", { textContent: d.speed_mbps ? speedWords(d.speed_mbps) : "Speed not reported" }))));
    if (!s.usb_devices.length) parts.push(el("p", { class: "muted", textContent: "No devices connected." }));
  }

  parts.push(findingList(r, id) || el("p", { class: "muted small", textContent: "Nothing to report for this reading." }));
  parts.push(el("p", { class: "source small", textContent: "Where these come from: the hardware reports them, or QloudiCable measures or works them out." }));
  return parts;
}

// Capabilities live under Tools, so the dashboard stays focused on live readings.
function renderCapabilities(s) {
  const c = s.capabilities;
  const rows = [["Cable chip", c.cable_identity], ["Charger profiles", c.charger_profiles], ["Link speed", c.link_speed], ["Power reading", c.power_measurement]];
  $("access").replaceChildren(...rows.map(([name, a]) => el("div", { class: "kv", title: a.why || "" }, el("span", { textContent: name }), el("strong", { textContent: ACCESS_WORD[a.level] }))));
  $("version").textContent = s.tool_version;
  $("tab-drive").hidden = s.platform === "android";
}

// ---- Selection and sheets ----------------------------------------------------

function selectTile(id) {
  const current = Store.get().selected;
  Store.set({ selected: current === id ? null : id });
  if (Store.get().selected && isPhone()) openDetailSheet();
}

const isPhone = () => window.matchMedia("(max-width: 820px)").matches;

function openDetailSheet() {
  $("dash").dataset.detail = "open";
  $("detail").focus?.();
}

let lastTrigger = null;

function openSheet(name, trigger) {
  const id = SHEETS[name];
  if (!id) return;
  lastTrigger = trigger || document.activeElement;
  for (const other of Object.values(SHEETS)) if (other !== id) $(other).hidden = true;
  $(id).hidden = false;
  $("scrim").hidden = false;
  const first = $(id).querySelector("button, [href], summary") || $(id);
  first.focus();
}

function closeSheet() {
  for (const id of Object.values(SHEETS)) $(id).hidden = true;
  $("scrim").hidden = true;
  if (lastTrigger && lastTrigger.focus) lastTrigger.focus();
}

// Dock: Overview clears the selection, Ports and Power select those readings,
// Tests and Tools open their sheets.
function setNav(active) {
  for (const id of ["nav-overview", "nav-ports", "nav-power"]) {
    const b = $(id);
    const on = id === active;
    b.setAttribute("aria-pressed", on ? "true" : "false");
    b.classList.toggle("on", on);
  }
}

// ---- Tests (unchanged behaviour; the test overlays are driven by features.js too) ----

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

// ---- Copy ---------------------------------------------------------------------

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

// ---- Data flow ----------------------------------------------------------------

const COMPONENTS = [Verdict, Tiles, Detail];

function apply(state) {
  for (const c of COMPONENTS) c.update(state);
  if (state.report) renderCapabilities(state.report.snapshot);
}

Store.subscribe(apply);

async function refresh() {
  const state = Store.get();
  if (state.busy) return;
  Store.set({ busy: true });
  try {
    const r = await call("report");
    $("error").hidden = true;
    $("updated").textContent = `Checked ${new Date().toLocaleTimeString()}`;
    const text = JSON.stringify(r);
    if (text !== state.lastJson) {
      const s = r.snapshot;
      if (s.power && s.power.milliwatts != null) Store.pushPower(s.power.milliwatts / 1000);
      Store.set({ report: r, lastJson: text });
    }
  } catch (e) {
    $("error").textContent = `Couldn't read this device: ${e}`;
    $("error").hidden = false;
  } finally {
    Store.set({ busy: false });
  }
}

// ---- Wiring -------------------------------------------------------------------

$("refresh").addEventListener("click", refresh);
$("copy").addEventListener("click", () => {
  const r = Store.get().report;
  if (r) copyText(summaryText(r), $("copy"), "Copied");
});
$("detail-close").addEventListener("click", () => {
  Store.set({ selected: null });
  setNav("nav-overview");
});
$("nav-overview").addEventListener("click", () => {
  Store.set({ selected: null });
  setNav("nav-overview");
});
$("nav-ports").addEventListener("click", () => {
  Store.set({ selected: "ports" });
  setNav("nav-ports");
  if (isPhone()) openDetailSheet();
});
$("nav-power").addEventListener("click", () => {
  Store.set({ selected: "power" });
  setNav("nav-power");
  if (isPhone()) openDetailSheet();
});
$("open-tests").addEventListener("click", (e) => openSheet("tests", e.currentTarget));
$("open-tools").addEventListener("click", (e) => openSheet("tools", e.currentTarget));
document.querySelectorAll("[data-open]").forEach((b) => b.addEventListener("click", (e) => openSheet(b.dataset.open, e.currentTarget)));
document.querySelectorAll("[data-close]").forEach((b) => b.addEventListener("click", closeSheet));
$("scrim").addEventListener("click", closeSheet);
$("charge-run").addEventListener("click", chargeTest);
$("write-run").addEventListener("click", writeTest);
$("qa-run").addEventListener("click", runSelfCheck);
$("copy-json").addEventListener("click", () => {
  const r = Store.get().report;
  if (r) copyText(JSON.stringify(r, null, 2), $("copy-json"), "Copied");
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    if (!$("scrim").hidden) closeSheet();
    else if (Store.get().selected && isPhone()) {
      Store.set({ selected: null });
      setNav("nav-overview");
    }
  }
});

// Desktop shows a reading from the start; phones start on the plain overview.
if (!isPhone()) Store.set({ selected: "power" });
Store.set({ report: null });
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
