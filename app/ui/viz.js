// Charts, motion and stats for QloudiCable, drawn as plain SVG so it runs under
// the app's content policy without libraries. Every visual sits next to a text
// equivalent, and motion is switched off when the system asks for less.
(function () {
  const NS = "http://www.w3.org/2000/svg";
  const calm = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function node(tag, attrs, parent) {
    const n = document.createElementNS(NS, tag);
    for (const k of Object.keys(attrs || {})) n.setAttribute(k, attrs[k]);
    if (parent) parent.appendChild(n);
    return n;
  }

  function reset(svg, viewBox, description) {
    svg.replaceChildren();
    svg.setAttribute("viewBox", viewBox);
    svg.setAttribute("role", "img");
    svg.setAttribute("aria-label", description);
  }

  function note(svg, text) {
    node("text", { x: "50%", y: "50%", "text-anchor": "middle", class: "viz-note" }, svg).textContent = text;
  }

  // Draws a path in from nothing, as if the line is being traced.
  function drawIn(shape, ms) {
    const len = shape.getTotalLength();
    shape.style.strokeDasharray = String(len);
    shape.style.strokeDashoffset = String(len);
    if (calm()) {
      shape.style.strokeDashoffset = "0";
      return;
    }
    shape.getBoundingClientRect(); // start the transition from the hidden state
    shape.style.transition = `stroke-dashoffset ${ms}ms ease-out`;
    shape.style.strokeDashoffset = "0";
  }

  // Counts a number up to its value. Shows the final value straight away when motion is off.
  function countUp(el, to, ms, fmt) {
    const show = fmt || ((v) => String(Math.round(v)));
    if (!Number.isFinite(to)) {
      el.textContent = "—";
      return;
    }
    if (calm()) {
      el.textContent = show(to);
      return;
    }
    const t0 = performance.now();
    const step = (now) => {
      const k = Math.min(1, (now - t0) / ms);
      el.textContent = show(to * (1 - Math.pow(1 - k, 3)));
      if (k < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }

  // Power over time. `values` are watts, one per reading.
  function lineChart(svg, values, opts) {
    const o = opts || {};
    const W = 600, H = 220, P = { l: 56, r: 16, t: 16, b: 30 };
    const unit = o.unit || "W";
    const summary = o.title || "Chart";
    reset(svg, `0 0 ${W} ${H}`, `${summary}: ${values.length} readings`);
    if (!values.length) return note(svg, "Waiting for readings…");

    const max = Math.max(o.max || 0, ...values) * 1.1 || 1;
    const x = (i) =>
      values.length === 1 ? (W + P.l - P.r) / 2 : P.l + (i / (values.length - 1)) * (W - P.l - P.r);
    const y = (v) => P.t + (1 - v / max) * (H - P.t - P.b);

    for (let g = 0; g <= 4; g++) {
      const v = (max * g) / 4;
      node("line", { x1: P.l, x2: W - P.r, y1: y(v), y2: y(v), class: "viz-grid" }, svg);
      node("text", { x: P.l - 8, y: y(v) + 4, "text-anchor": "end", class: "viz-axis" }, svg).textContent =
        `${v.toFixed(0)} ${unit}`;
    }
    const d = values.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
    const base = H - P.b;
    node("path", { d: `${d} L${x(values.length - 1)},${base} L${x(0)},${base} Z`, class: "viz-area" }, svg);
    drawIn(node("path", { d, class: "viz-line" }, svg), 900);
    if (o.average != null) {
      node("line", { x1: P.l, x2: W - P.r, y1: y(o.average), y2: y(o.average), class: "viz-avg" }, svg);
    }
    values.forEach((v, i) => node("circle", { cx: x(i), cy: y(v), r: 3.5, class: "viz-dot" }, svg));
  }

  // One row per port; filled bars are readings where the port was connected.
  function timeline(svg, watch) {
    const ports = [...new Set(watch.samples.map((s) => s.port_id))];
    const alerted = new Set(watch.alerts.map((a) => a.port_id));
    const W = 600, rowH = 40, left = 150, right = 16, top = 12;
    const H = top + Math.max(1, ports.length) * rowH + 28;
    reset(svg, `0 0 ${W} ${H}`, `Connection timeline for ${ports.length} port(s). ${watch.summary}`);
    if (!ports.length) return note(svg, "No ports were reported.");

    const times = watch.samples.map((s) => s.t_ms);
    const t0 = Math.min(...times);
    const t1 = Math.max(...times) + watch.sample_interval_ms;
    const span = Math.max(1, t1 - t0);
    const x = (t) => left + ((t - t0) / span) * (W - left - right);

    ports.forEach((port, row) => {
      const y = top + row * rowH;
      if (alerted.has(port)) node("rect", { x: 0, y: y + 2, width: W, height: rowH - 4, rx: 8, class: "viz-alert-box" }, svg);
      const short = port.length > 18 ? `${port.slice(0, 17)}…` : port;
      node("text", { x: 8, y: y + rowH / 2 + 5, class: alerted.has(port) ? "viz-axis viz-alert-text" : "viz-axis" }, svg).textContent = short;
      watch.samples
        .filter((s) => s.port_id === port)
        .sort((a, b) => a.t_ms - b.t_ms)
        .forEach((s, i) => {
          const x0 = x(s.t_ms), x1 = x(s.t_ms + watch.sample_interval_ms);
          const bar = node("rect", { x: x0, y: y + 10, width: Math.max(1, x1 - x0), height: rowH - 20, rx: 2, class: s.connected ? "viz-on" : "viz-off" }, svg);
          if (!calm()) {
            bar.style.animation = "viz-in 300ms ease-out both";
            bar.style.animationDelay = `${Math.min(i * 3, 800)}ms`;
          }
        });
    });
    for (let s = 0; s <= Math.floor(span / 1000); s++) {
      node("text", { x: x(t0 + s * 1000), y: H - 6, "text-anchor": "middle", class: "viz-axis" }, svg).textContent = `${s}s`;
    }
  }

  // A ring that fills to a percentage.
  function ring(svg, pct, text) {
    const v = Math.max(0, Math.min(100, Number(pct) || 0));
    reset(svg, "0 0 120 120", `${text}: ${v} out of 100`);
    const r = 50, c = 2 * Math.PI * r;
    node("circle", { cx: 60, cy: 60, r, class: "viz-ring-bg" }, svg);
    const fg = node("circle", { cx: 60, cy: 60, r, class: "viz-ring-fg", transform: "rotate(-90 60 60)" }, svg);
    fg.style.strokeDasharray = String(c);
    const target = String(c * (1 - v / 100));
    fg.style.strokeDashoffset = String(c);
    if (calm()) fg.style.strokeDashoffset = target;
    else
      requestAnimationFrame(() => {
        fg.style.transition = "stroke-dashoffset 900ms ease-out";
        fg.style.strokeDashoffset = target;
      });
    countUp(node("text", { x: 60, y: 68, "text-anchor": "middle", class: "viz-ring-text" }, svg), v, 900);
  }

  // The tested end in the port, with its e-marker chip, and the other end.
  // `state.emarker` is true, false, or null when not tested yet.
  function cable(svg, state) {
    const label = state.emarker === null ? "not tested" : state.emarker ? "e-marker answered" : "no e-marker answer";
    const other = state.end === "A" ? "B" : "A";
    reset(svg, "0 0 600 150", `End ${state.end} in the port, ${state.step === 2 ? "turned 180 degrees" : "as inserted"}: ${label}`);

    node("rect", { x: 10, y: 54, width: 100, height: 52, rx: 10, class: "viz-device" }, svg);
    node("text", { x: 60, y: 86, "text-anchor": "middle", class: "viz-label" }, svg).textContent = "Port";
    node("text", { x: 150, y: 44, "text-anchor": "middle", class: "viz-label" }, svg).textContent = `End ${state.end}`;

    const plug = node("g", { class: "viz-plug-group" }, svg);
    node("rect", { x: 120, y: 62, width: 60, height: 36, rx: 6, class: "viz-plug tested" }, plug);
    node("rect", { x: 138, y: 72, width: 24, height: 16, rx: 3, class: state.emarker ? "viz-chip-on" : "viz-chip-off" }, plug);
    plug.style.transformOrigin = "150px 80px";
    plug.style.transition = calm() ? "none" : "transform 700ms ease-in-out";
    plug.style.transform = "rotate(0deg)";
    if (state.step === 2) requestAnimationFrame(() => (plug.style.transform = "rotate(180deg)"));

    const live = !!state.live;
    node("path", { d: "M180,80 L420,80", class: "viz-wire" }, svg);
    if (live) node("path", { d: "M180,80 L420,80", class: "viz-flow" }, svg);

    node("rect", { x: 420, y: 62, width: 60, height: 36, rx: 6, class: "viz-plug" }, svg);
    node("text", { x: 450, y: 44, "text-anchor": "middle", class: "viz-label" }, svg).textContent = `End ${other}`;
    node("rect", { x: 490, y: 54, width: 100, height: 52, rx: 10, class: "viz-device" }, svg);
    node("text", { x: 540, y: 86, "text-anchor": "middle", class: "viz-label" }, svg).textContent = "Other end";
    node("text", { x: 300, y: 130, "text-anchor": "middle", class: "viz-label" }, svg).textContent = label;
  }

  // The numbers behind a chart, for screen readers and anyone who wants them.
  function dataTable(svg, caption, rows) {
    let box = svg.nextElementSibling;
    if (!box || !box.classList.contains("data-table")) {
      box = document.createElement("details");
      box.className = "data-table";
      svg.after(box);
    }
    const summary = document.createElement("summary");
    summary.textContent = `Show the numbers: ${caption}`;
    const table = document.createElement("table");
    const body = document.createElement("tbody");
    for (const [label, value] of rows) {
      const tr = document.createElement("tr");
      const th = document.createElement("th");
      th.scope = "row";
      th.textContent = label;
      const td = document.createElement("td");
      td.textContent = value;
      tr.append(th, td);
      body.append(tr);
    }
    table.append(body);
    box.replaceChildren(summary, table);
  }

  // Big numbers that count up.
  function stats(container, items) {
    container.replaceChildren();
    for (const it of items) {
      const value = document.createElement("span");
      value.className = "stat-value";
      if (typeof it.value === "number") countUp(value, it.value, 700, it.fmt);
      else value.textContent = it.value;
      const label = document.createElement("span");
      label.className = "stat-label";
      label.textContent = it.label;
      const box = document.createElement("div");
      box.className = "stat";
      box.append(value, label);
      container.append(box);
    }
  }

  window.QCViz = { lineChart, timeline, ring, cable, stats, countUp, dataTable };
})();
