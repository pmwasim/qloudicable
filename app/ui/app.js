// QloudiCable front end. The Rust core decides every result. This file only shows
// the result in plain words, and never invents a verdict of its own.
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
const CABLE_KIND = { passive: "Passive", active: "Active", vpd: "Powered device", unknown: "Unknown" };
const CHARGE_STATE = {
  charging: "Charging",
  discharging: "On battery",
  full: "Battery full",
  not_charging: "Plugged in, not charging",
  unknown: "Unknown",
};
const SEVERITY_WORD = { good: "Good", info: "Note", warn: "Check", problem: "Problem" };
const SEVERITY_GLYPH = { good: "✓", info: "i", warn: "!", problem: "✕" };
const EVIDENCE = {
  reported: "Reported by the hardware",
  measured: "Measured",
  inferred: "Worked out from the USB spec",
};
const ACCESS_WORD = { full: "Yes", partial: "Partly", none: "No" };

let report = null;
let lastJson = null;
let busy = false;
let showAllFindings = false;
let showAllDevices = false;

// ---- Hero: one answer, one action

function heroFor(r) {
  const s = r.snapshot;
  const worst = r.findings[0];
  const hasDevices = s.usb_devices.some((d) => !d.is_hub);
  const connected = s.ports.some((p) => p.connected);
  const blind = s.capabilities.cable_identity.level === "none" && !s.power;

  if (worst && (worst.severity === "problem" || worst.severity === "warn")) {
    const cableRelated = worst.code.startsWith("cable") || worst.code === "unusual_emarker";
    return {
      level: worst.severity,
      status: SEVERITY_WORD[worst.severity],
      title: worst.title,
      text: worst.detail,
      action: cableRelated ? "guide" : "watch",
    };
  }
  if (worst && worst.severity === "good") {
    return { level: "good", status: "All clear", title: "No problems found", text: worst.detail, action: null };
  }
  if (connected || hasDevices) {
    return {
      level: "good",
      status: "All clear",
      title: "No problems found",
      text: "Everything this device reports looks normal.",
      action: null,
    };
  }
  if (blind) {
    return {
      level: "info",
      status: "Limited view",
      title: "This device can't see cable details",
      text: "It can still show devices and power. Use the tests below to check a cable yourself.",
      action: "charge",
    };
  }
  return {
    level: "idle",
    status: "Waiting",
    title: "Plug in a cable to start",
    text: "Connect a charger or device. This page updates by itself.",
    action: "watch",
  };
}

const ACTION_LABEL = {
  guide: "Run the both-ends test",
  watch: "Start a port watch",
  charge: "Start a charging test",
};
const ACTION_TAB = { guide: "guide", watch: "watch", charge: "charge" };

function renderHero(r) {
  const h = heroFor(r);
  const hero = $("hero");
  hero.dataset.level = h.level;
  $("hero-status").textContent = h.status;
  $("hero-title").textContent = h.title;
  $("hero-text").textContent = h.text;
  const btn = $("hero-action");
  btn.hidden = !h.action;
  if (h.action) {
    btn.textContent = ACTION_LABEL[h.action];
    btn.onclick = () => {
      showTab(ACTION_TAB[h.action], true);
      $("tools-anchor").scrollIntoView({ behavior: reduceMotion() ? "auto" : "smooth", block: "start" });
    };
  }
}

// ---- Findings: top three, then "show all"

function renderFindings(r) {
  const list = r.findings;
  $("findings-box").hidden = list.length === 0;
  const shown = showAllFindings ? list : list.slice(0, 3);
  $("findings").replaceChildren(
    ...shown.map((f) =>
      el(
        "li",
        { class: `finding ${f.severity}` },
        el("span", { class: "glyph", "aria-hidden": "true", textContent: SEVERITY_GLYPH[f.severity] }),
        el(
          "div",
          {},
          el(
            "p",
            { class: "f-title" },
            el("span", { class: "sr-only", textContent: `${SEVERITY_WORD[f.severity]}: ` }),
            f.title
          ),
          el("p", { class: "f-detail", textContent: f.detail }),
          el("span", { class: "tag", textContent: EVIDENCE[f.evidence] })
        )
      )
    )
  );
  const more = $("findings-more");
  more.hidden = list.length <= 3;
  more.textContent = showAllFindings ? "Show fewer" : `Show all ${list.length} findings`;
  more.setAttribute("aria-expanded", showAllFindings ? "true" : "false");
}

// ---- Ports

function grid(pairs) {
  const dl = el("dl", { class: "facts" });
  for (const [k, v] of pairs) if (v) dl.append(el("dt", { textContent: k }), el("dd", { textContent: v }));
  return dl;
}

function portCard(p, identity) {
  const card = el(
    "article",
    { class: p.connected ? "card port connected" : "card port" },
    el(
      "div",
      { class: "port-head" },
      el("h3", { textContent: p.label }),
      el("span", { class: p.connected ? "chip on" : "chip", textContent: p.connected ? "Connected" : "Empty" })
    )
  );
  if (!p.connected) {
    card.append(el("p", { class: "muted", textContent: "Nothing plugged in." }));
    return card;
  }

  const facts = [];
  if (p.cable) {
    const power = p.cable.max_watts ? `${p.cable.max_watts} W` : "power not stated";
    facts.push(["Cable", `${CABLE_SPEED[p.cable.speed]}, ${power}`]);
    facts.push(["Maker", p.cable.vendor_name || `Unknown (${hex(p.cable.vendor_id, 4)})`]);
  } else if (identity === "full") {
    facts.push(["Cable", "No e-marker: limited to 60 W"]);
  }
  if (p.partner && (p.partner.kind || p.partner.vendor_name)) {
    facts.push(["Connected to", [p.partner.vendor_name, p.partner.kind].filter(Boolean).join(", ")]);
  }
  if (p.transports.length) facts.push(["In use", p.transports.join(", ")]);
  card.append(grid(facts));

  if (p.source_pdos.length) {
    const active = p.active_pdo && p.active_pdo.max_mv;
    card.append(
      el("p", { class: "pdo", textContent: "Charger offers" }),
      el(
        "div",
        { class: "pdo-list" },
        ...p.source_pdos.map((o) => {
          const volts = o.min_mv === o.max_mv ? `${o.max_mv / 1000} V` : `${o.min_mv / 1000} to ${o.max_mv / 1000} V`;
          return el("span", {
            class: o.max_mv === active ? "pdo-chip active" : "pdo-chip",
            textContent: `${volts}, ${watts(o.max_mw)}`,
          });
        })
      )
    );
  }

  const raw = [
    ["Data role", p.data_role],
    ["Power role", p.power_role],
    ["Plug side", p.orientation],
  ];
  const more = el("details", { class: "more" }, el("summary", { textContent: "More about this port" }), grid(raw));
  if (p.cable) {
    more.append(el("code", { textContent: p.cable.raw_vdos.map((v) => "0x" + hex(v, 8)).join(" ") }));
  }
  card.append(more);
  return card;
}

function renderPorts(s) {
  const identity = s.capabilities.cable_identity.level;
  $("ports").replaceChildren(
    ...(s.ports.length
      ? s.ports.map((p) => portCard(p, identity))
      : [el("article", { class: "card" }, el("p", { class: "muted", textContent: `No USB-C ports are reported here. ${s.capabilities.cable_identity.why}` }))])
  );
}

// ---- Power

function renderPower(s) {
  const p = s.power;
  $("power-box").hidden = !p;
  if (!p) return;
  const reading = p.milliwatts != null ? watts(p.milliwatts) : "No reading";
  $("power").replaceChildren(
    el("div", { class: "big", textContent: reading }),
    el("p", { class: "muted", textContent: CHARGE_STATE[p.state] || CHARGE_STATE.unknown }),
    el(
      "div",
      { class: "stats" },
      ...[
        ["Battery", p.battery_percent != null ? `${p.battery_percent}%` : "Not reported"],
        ["Voltage", p.voltage_mv ? `${(p.voltage_mv / 1000).toFixed(2)} V` : "Not reported"],
        ["Current", p.current_ma ? `${(p.current_ma / 1000).toFixed(2)} A` : "Not reported"],
      ].map(([label, value]) =>
        el("div", { class: "stat" }, el("span", { class: "stat-value", textContent: value }), el("span", { class: "stat-label", textContent: label }))
      )
    ),
    el("p", { class: "muted", textContent: `Source: ${p.source}` })
  );
}

// ---- Devices

function renderDevices(s) {
  const list = s.usb_devices.filter((d) => !d.is_hub);
  $("devices-box").hidden = list.length === 0;
  const shown = showAllDevices ? list : list.slice(0, 5);
  $("devices").replaceChildren(
    ...shown.map((d) => {
      const name = d.product_name || d.vendor_name || "Unnamed device";
      const sub = [d.vendor_name && d.product_name ? d.vendor_name : null, d.usb_version ? `USB ${d.usb_version}` : null]
        .filter(Boolean)
        .join(" · ");
      return el(
        "div",
        { class: "row" },
        el("div", {}, name, el("span", { class: "sub", textContent: sub || `${hex(d.vendor_id, 4)}:${hex(d.product_id, 4)}` })),
        el("div", { class: "val", textContent: d.speed_mbps ? speedWords(d.speed_mbps) : "Speed not reported" })
      );
    })
  );
  const more = $("devices-more");
  more.hidden = list.length <= 5;
  more.textContent = showAllDevices ? "Show fewer" : `Show all ${list.length} devices`;
  more.setAttribute("aria-expanded", showAllDevices ? "true" : "false");
}

// ---- What this device can show

function renderAccess(s) {
  const c = s.capabilities;
  const rows = [
    ["Cable chip (e-marker)", c.cable_identity],
    ["Charger power profiles", c.charger_profiles],
    ["USB link speed", c.link_speed],
    ["Power measurement", c.power_measurement],
  ];
  $("access").replaceChildren(
    ...rows.map(([name, a]) =>
      el(
        "div",
        { class: "row" },
        el("div", {}, name, a.why ? el("span", { class: "sub", textContent: a.why }) : null),
        el("div", { class: "val", textContent: ACCESS_WORD[a.level] })
      )
    )
  );
  $("version").textContent = s.tool_version;
  // Drive speed is a desktop test. Hide its tab on phones.
  const android = s.platform === "android";
  $("tab-drive").hidden = android;
  if (android && $("tab-drive").getAttribute("aria-selected") === "true") showTab("charge");
}

// ---- Tests: tabs

const TABS = ["charge", "watch", "guide", "drive"];

function showTab(name, focus) {
  for (const t of TABS) {
    const tab = $(`tab-${t}`);
    const panel = $(`panel-${t}`);
    const on = t === name;
    tab.setAttribute("aria-selected", on ? "true" : "false");
    tab.tabIndex = on ? 0 : -1;
    panel.hidden = !on;
  }
  if (focus) $(`tab-${name}`).focus();
}

function visibleTabs() {
  return TABS.filter((t) => !$(`tab-${t}`).hidden);
}

function onTabKey(e) {
  const order = visibleTabs();
  const current = order.indexOf(TABS.find((t) => $(`tab-${t}`) === e.currentTarget));
  let next = null;
  if (e.key === "ArrowRight") next = order[(current + 1) % order.length];
  if (e.key === "ArrowLeft") next = order[(current - 1 + order.length) % order.length];
  if (e.key === "Home") next = order[0];
  if (e.key === "End") next = order[order.length - 1];
  if (next) {
    e.preventDefault();
    showTab(next, true);
  }
}

// ---- Tests: charge and drive speed

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

// ---- Self-check and copy

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

function summaryText(r) {
  const s = r.snapshot;
  const h = heroFor(r);
  const lines = [`QloudiCable on ${s.platform}: ${h.title}.`];
  for (const p of s.ports.filter((x) => x.connected)) {
    const cable = p.cable
      ? `${CABLE_SPEED[p.cable.speed]}, ${p.cable.max_watts ? p.cable.max_watts + " W" : "power not stated"}`
      : "no cable detail";
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
  renderHero(r);
  renderFindings(r);
  renderPorts(s);
  renderPower(s);
  renderDevices(s);
  renderAccess(s);
}

async function refresh() {
  if (busy) return;
  busy = true;
  try {
    const r = await call("report");
    $("error").hidden = true;
    const text = JSON.stringify(r);
    if (text !== lastJson) {
      // Redraw only on change, so open details and focus stay put.
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
$("copy").addEventListener("click", () => {
  if (report) copyText(summaryText(report), $("copy"), "Copied");
});
$("copy-json").addEventListener("click", () => {
  if (report) copyText(JSON.stringify(report, null, 2), $("copy-json"), "Copied");
});
$("findings-more").addEventListener("click", () => {
  showAllFindings = !showAllFindings;
  if (report) renderFindings(report);
});
$("devices-more").addEventListener("click", () => {
  showAllDevices = !showAllDevices;
  if (report) renderDevices(report.snapshot);
});
$("charge-run").addEventListener("click", chargeTest);
$("write-run").addEventListener("click", writeTest);
$("qa-run").addEventListener("click", runSelfCheck);
for (const t of TABS) {
  const tab = $(`tab-${t}`);
  tab.addEventListener("click", () => showTab(t));
  tab.addEventListener("keydown", onTabKey);
}
// Anchor for "scroll to tests" from the hero.
const anchor = el("span", { id: "tools-anchor" });
$("tools-title").before(anchor);

showTab("charge");
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
