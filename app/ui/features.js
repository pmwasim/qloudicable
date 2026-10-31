// The user-facing features built on the app: the port watch, the both-ends guide,
// the charge chart. Rules live in the
// Rust core. This file asks for results and draws them.
(function () {
  const V = window.QCViz;
  const POSITIONS = [
    ["A", 1],
    ["A", 2],
    ["B", 1],
    ["B", 2],
  ];
  const TURN = { 1: "as inserted", 2: "turned 180 degrees" };
  const STATE_WORDS = { pass: "E-marker answered", no_emarker: "No e-marker answer", not_tested: "Not tested" };
  const STATE_ICON = { pass: "✓", no_emarker: "!", not_tested: "–" };

  let runs = [];
  let selected = 0;
  let busy = false;

  const positionName = (end, step) => `End ${end}, ${TURN[step]}`;
  const latestRun = (end, step) => runs.slice().reverse().find((r) => r.end === end && r.step === step);

  function nextUntested() {
    const i = POSITIONS.findIndex(([end, step]) => {
      const r = latestRun(end, step);
      return !r || !r.connected;
    });
    return i === -1 ? selected : i;
  }

  // ---- Both-ends guide

  function renderSteps() {
    $("guide-steps").replaceChildren(
      ...POSITIONS.map(([end, step], i) => {
        const r = latestRun(end, step);
        const state = !r ? "not tested" : !r.connected ? "not connected" : r.emarker_seen ? "e-marker answered" : "no e-marker answer";
        const b = document.createElement("button");
        b.type = "button";
        b.className = i === selected ? "step-chip selected" : "step-chip";
        b.setAttribute("aria-pressed", i === selected ? "true" : "false");
        b.textContent = `End ${end}, ${TURN[step]}: ${state}`;
        b.addEventListener("click", () => {
          selected = i;
          renderGuide();
        });
        return b;
      })
    );
  }

  function renderGuide() {
    renderSteps();
    const [end, step] = POSITIONS[selected];
    const r = latestRun(end, step);
    V.cable($("guide-cable"), {
      end,
      step,
      emarker: r && r.connected ? r.emarker_seen : null,
      live: !!(r && r.connected),
    });
    $("guide-text").textContent = `Plug End ${end} into the port${step === 2 ? ", turned 180 degrees" : ""}. Then press Record.`;
  }

  function renderMatrix(m) {
    $("guide-matrix").replaceChildren(
      ...m.cells.map((c) => {
        const key = c.state.replace("_", "-");
        const card = document.createElement("div");
        card.className = `cell ${key}`;
        const head = document.createElement("strong");
        head.textContent = positionName(c.end, c.step);
        const state = document.createElement("span");
        state.className = "cell-state";
        state.textContent = `${STATE_ICON[c.state]} ${STATE_WORDS[c.state]}`;
        card.append(head, state);
        if (c.charge_mw != null) {
          const charge = document.createElement("span");
          charge.className = "cell-charge";
          charge.textContent = `Charging ${(c.charge_mw / 1000).toFixed(1)} W`;
          card.append(charge);
        }
        return card;
      })
    );
  }

  async function record() {
    if (busy) return;
    busy = true;
    const [end, step] = POSITIONS[selected];
    $("guide-record").disabled = true;
    try {
      runs.push(await call("capture_run", { end, step }));
      const m = await call("orientation_matrix", { runs });
      renderMatrix(m);
      $("guide-verdict").textContent = m.verdict;
      selected = nextUntested();
      renderGuide();
    } catch (e) {
      $("guide-verdict").textContent = `Couldn't record this position: ${e}`;
    } finally {
      busy = false;
      $("guide-record").disabled = false;
    }
  }

  function resetGuide() {
    runs = [];
    selected = 0;
    $("guide-matrix").replaceChildren(el("p", { class: "note", textContent: "Record the four positions to see the result." }));
    $("guide-verdict").textContent = "";
    renderGuide();
  }

  // ---- Port watch

  async function runWatch() {
    const btn = $("watch-run");
    const out = $("watch-out");
    btn.disabled = true;
    btn.textContent = "Watching…";
    out.classList.remove("alert");
    out.textContent = "Watching for 10 seconds. Plug and unplug now if you're testing a loose connection.";
    try {
      const w = await call("watch_ports", { seconds: 10 });
      V.timeline($("watch-timeline"), w);
      const ports = [...new Set(w.samples.map((s) => s.port_id))];
      V.stats(
        $("watch-stats"),
        [
          { value: ports.length, fmt: (v) => String(Math.round(v)), label: "Ports watched" },
          { value: w.alerts.length, fmt: (v) => String(Math.round(v)), label: "Cycling ports" },
          {
            value: Math.round(
              (100 * w.samples.filter((s) => s.connected).length) / Math.max(1, w.samples.length)
            ),
            fmt: (v) => `${Math.round(v)}%`,
            label: "Time connected",
          },
        ]
      );
      out.textContent = w.summary;
      out.classList.toggle("alert", w.alerts.length > 0);
    } catch (e) {
      out.textContent = `Watch failed: ${e}`;
    } finally {
      btn.disabled = false;
      btn.textContent = "Watch ports for 10 s";
    }
  }

  // ---- Charge chart, driven from app.js

  function live(samples) {
    V.lineChart($("charge-chart"), samples, { title: "Charging power", unit: "W", max: 100 });
  }

  function charged(samples, t) {
    V.lineChart($("charge-chart"), samples, { title: "Charging power", unit: "W", max: 100, average: t.avg_w });
    V.stats($("charge-stats"), [
      { value: t.avg_w, fmt: (v) => `${v.toFixed(1)} W`, label: "Average" },
      { value: t.min_w, fmt: (v) => `${v.toFixed(1)} W`, label: "Lowest" },
      { value: t.max_w, fmt: (v) => `${v.toFixed(1)} W`, label: "Highest" },
      { value: t.samples, fmt: (v) => String(Math.round(v)), label: "Readings" },
    ]);
  }

  async function init() {
    $("watch-run").addEventListener("click", runWatch);
    $("guide-record").addEventListener("click", record);
    $("guide-reset").addEventListener("click", resetGuide);
    resetGuide();
  }

  window.QCFeatures = { live, charged };
  init();
})();
