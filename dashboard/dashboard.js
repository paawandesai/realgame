// @ts-check
// Analytics dashboard. Reads /api/stats with the DASHBOARD_TOKEN the viewer
// enters once (kept in sessionStorage for this tab only) and draws every chart
// itself as SVG: no third-party code, no inline scripts or styles, so it runs
// under the game's strict Content Security Policy.

const TOKEN_KEY = "rg.dashboardToken";
const SVG = "http://www.w3.org/2000/svg";
/** @type {readonly ["sprout", "seedling", "apprentice"]} */
const BANDS = ["sprout", "seedling", "apprentice"];
/** @type {Record<string, string>} */
const BAND_LABEL = { sprout: "Sprout (4-7)", seedling: "Seedling (8-10)", apprentice: "Apprentice (11+)" };
/** @type {Record<string, string>} */
const DURATION_LABEL = {
  lt1m: "Under 1 min",
  "1to5m": "1-5 min",
  "5to15m": "5-15 min",
  "15to30m": "15-30 min",
  "30mplus": "30 min +",
};

/**
 * @typedef {{ sprout: number, seedling: number, apprentice: number }} BandCounts
 * @typedef {BandCounts & { all: number }} BandCountsAll
 * @typedef {{
 *   generatedAt: string, today: string, windowDays: number, retentionDays: number,
 *   players: { today: number, todayByBand: BandCounts, playerDays: number, playerDaysByBand: BandCounts },
 *   sessions: { today: number, total: number },
 *   daily: { day: string, players: number, sessions: number, byBand: BandCounts }[],
 *   funnel: {
 *     started: BandCountsAll,
 *     quests: (BandCountsAll & { quest: number })[],
 *     rooms: (BandCountsAll & { room: number })[],
 *     trialComplete: BandCountsAll,
 *   },
 *   durations: { bucket: string, players: number }[],
 * }} Stats
 */

/**
 * @template {HTMLElement} T
 * @param {string} id
 * @returns {T}
 */
function byId(id) {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} missing`);
  return /** @type {T} */ (el);
}

/** @returns {string | null} */
function getToken() {
  try {
    return window.sessionStorage.getItem(TOKEN_KEY);
  } catch {
    return memoryToken;
  }
}
/** @type {string | null} */
let memoryToken = null;
/** @param {string | null} token */
function setToken(token) {
  memoryToken = token;
  try {
    if (token === null) window.sessionStorage.removeItem(TOKEN_KEY);
    else window.sessionStorage.setItem(TOKEN_KEY, token);
  } catch {
    // Storage blocked: the token lives in memory until the page reloads.
  }
}

/**
 * @param {string} text
 * @param {boolean} [error]
 */
function setStatus(text, error = false) {
  const el = byId("status");
  el.textContent = text;
  el.classList.toggle("error", error);
}

const fmt = new Intl.NumberFormat("en");
/** @param {number} n */
const num = (n) => fmt.format(n);
/** @param {number} part @param {number} whole */
const pct = (part, whole) => (whole > 0 ? `${Math.round((part / whole) * 100)}%` : "–");

// -------------------------------------------------------------- svg helpers

/**
 * @param {string} tag
 * @param {Record<string, string | number>} [attrs]
 * @param {string} [text]
 * @returns {SVGElement}
 */
function svgEl(tag, attrs = {}, text) {
  const el = /** @type {SVGElement} */ (document.createElementNS(SVG, tag));
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  if (text !== undefined) el.textContent = text;
  return el;
}

/**
 * A bar with a 4px rounded data end and a square baseline end.
 * @param {number} x @param {number} y @param {number} w @param {number} h
 * @param {"top" | "right" | "none"} round
 */
function barPath(x, y, w, h, round) {
  if (w <= 0 || h <= 0) return "";
  const r = Math.min(4, round === "top" ? w / 2 : h / 2, round === "top" ? h : w);
  if (round === "top") {
    return `M${x},${y + h}V${y + r}Q${x},${y} ${x + r},${y}H${x + w - r}Q${x + w},${y} ${x + w},${y + r}V${y + h}Z`;
  }
  if (round === "right") {
    return `M${x},${y}H${x + w - r}Q${x + w},${y} ${x + w},${y + r}V${y + h - r}Q${x + w},${y + h} ${x + w - r},${y + h}H${x}Z`;
  }
  return `M${x},${y}H${x + w}V${y + h}H${x}Z`;
}

/** A "nice" axis maximum and tick step for `max`. @param {number} max */
function niceScale(max) {
  if (max <= 0) return { top: 4, step: 1 };
  const raw = max / 4;
  const mag = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * mag).find((s) => s >= raw) ?? raw;
  const s = Math.max(1, step);
  return { top: Math.ceil(max / s) * s, step: s };
}

// ----------------------------------------------------------------- tooltip

const tooltip = () => byId("tooltip");

/**
 * @param {Element} target
 * @param {string} title
 * @param {[string, string][]} rows
 */
function attachTooltip(target, title, rows) {
  /** @param {number} x @param {number} y */
  const show = (x, y) => {
    const tip = tooltip();
    const head = document.createElement("strong");
    head.textContent = title;
    const lines = rows.map(([k, v]) => {
      const line = document.createElement("div");
      const a = document.createElement("span");
      a.textContent = k;
      const b = document.createElement("span");
      b.textContent = v;
      line.append(a, b);
      return line;
    });
    tip.replaceChildren(head, ...lines);
    tip.hidden = false;
    const w = tip.offsetWidth;
    const h = tip.offsetHeight;
    tip.style.left = `${Math.min(window.innerWidth - w - 8, Math.max(8, x + 12))}px`;
    tip.style.top = `${Math.max(8, y - h - 12)}px`;
  };
  target.addEventListener("pointermove", (e) => {
    const ev = /** @type {PointerEvent} */ (e);
    show(ev.clientX, ev.clientY);
  });
  target.addEventListener("pointerleave", () => (tooltip().hidden = true));
  target.addEventListener("focus", () => {
    const r = target.getBoundingClientRect();
    show(r.left + r.width / 2, r.top);
  });
  target.addEventListener("blur", () => (tooltip().hidden = true));
}

// ------------------------------------------------------------------ charts

/** Daily unique players, stacked by band. @param {Stats} s */
function drawDaily(s) {
  const host = byId("chart-daily");
  // Drawn at the host's real pixel width, so text stays 12px at any size.
  const W = Math.max(280, host.clientWidth);
  const H = 240;
  const m = { top: 12, right: 4, bottom: 28, left: 32 };
  const iw = W - m.left - m.right;
  const ih = H - m.top - m.bottom;
  const { top, step } = niceScale(Math.max(...s.daily.map((d) => d.players)));
  const svg = svgEl("svg", { viewBox: `0 0 ${W} ${H}`, width: W, height: H, role: "img", "aria-label": "Daily unique players, last 30 days, stacked by age band" });

  for (let v = 0; v <= top; v += step) {
    const y = m.top + ih - (v / top) * ih;
    svg.append(svgEl("line", { class: v === 0 ? "axis" : "gridline", x1: m.left, x2: W - m.right, y1: y, y2: y }));
    svg.append(svgEl("text", { x: m.left - 6, y: y + 4, "text-anchor": "end" }, num(v)));
  }

  const slot = iw / s.daily.length;
  const bw = Math.max(2, slot - Math.min(6, slot * 0.25));
  s.daily.forEach((d, i) => {
    const x = m.left + i * slot + (slot - bw) / 2;
    const g = svgEl("g");
    let y = m.top + ih;
    // Bands stack in fixed order; the band total can exceed `players` when a
    // device switched band that day, so stack by band and scale to its sum.
    const bandSum = BANDS.reduce((acc, b) => acc + d.byBand[b], 0);
    const shown = BANDS.filter((b) => d.byBand[b] > 0);
    shown.forEach((b, j) => {
      const h = (d.byBand[b] / top) * ih;
      const isTop = j === shown.length - 1;
      // 2px surface gap between stacked segments.
      const gap = isTop ? 0 : 2;
      const p = barPath(x, y - h, bw, Math.max(0, h - gap), isTop ? "top" : "none");
      if (p) g.append(svgEl("path", { d: p, class: `fill-${b}` }));
      y -= h;
    });
    if (bandSum === 0 && d.players > 0) {
      const h = (d.players / top) * ih;
      g.append(svgEl("path", { d: barPath(x, m.top + ih - h, bw, h, "top"), class: "fill-bar" }));
    }
    const hit = svgEl("rect", { class: "hit", x: m.left + i * slot, y: m.top, width: slot, height: ih, tabindex: 0 });
    attachTooltip(hit, `${d.day}`, [
      ["Players", num(d.players)],
      ["Sessions", num(d.sessions)],
      ...BANDS.map((b) => /** @type {[string, string]} */ ([BAND_LABEL[b] ?? b, num(d.byBand[b])])),
    ]);
    hit.addEventListener("pointerenter", () => g.classList.add("hover"));
    hit.addEventListener("pointerleave", () => g.classList.remove("hover"));
    svg.append(g, hit);
    // Label every 7th day (14th on narrow screens), counting back from today.
    if ((s.daily.length - 1 - i) % (W < 600 ? 14 : 7) === 0) {
      svg.append(svgEl("text", { x: x + bw / 2, y: H - 8, "text-anchor": "middle" }, d.day.slice(5)));
    }
  });
  host.replaceChildren(svg);

  const legend = byId("legend-daily");
  legend.replaceChildren(
    ...BANDS.map((b) => {
      const item = document.createElement("span");
      const sw = document.createElement("i");
      sw.className = `swatch fill-${b}`;
      item.append(sw, BAND_LABEL[b] ?? b);
      return item;
    }),
  );
}

/**
 * Horizontal bars as HTML rows (label, bar, value), so text keeps its size
 * and wraps on narrow screens. Bar widths are set through the CSSOM, which
 * the CSP allows (it blocks only style attributes in markup).
 * @param {HTMLElement} host
 * @param {string} aria
 * @param {{ label: string, value: number, cls: string, note?: string, tip: [string, string][] }[]} rows
 */
function drawHBars(host, aria, rows) {
  const max = Math.max(1, ...rows.map((r) => r.value));
  const list = document.createElement("div");
  list.className = "hbars";
  list.setAttribute("role", "list");
  list.setAttribute("aria-label", aria);
  for (const r of rows) {
    const row = document.createElement("div");
    row.className = "hbar";
    row.setAttribute("role", "listitem");
    row.tabIndex = 0;
    const label = document.createElement("span");
    label.className = "hbar-label";
    label.textContent = r.label;
    const track = document.createElement("span");
    track.className = "hbar-track";
    const bar = document.createElement("span");
    bar.className = `hbar-fill ${r.cls}`;
    bar.style.width = `${(r.value / max) * 100}%`;
    track.append(bar);
    const value = document.createElement("span");
    value.className = "hbar-value";
    value.textContent = num(r.value);
    if (r.note) {
      const note = document.createElement("span");
      note.className = "hbar-note";
      note.textContent = r.note;
      value.append(note);
    }
    row.append(label, track, value);
    attachTooltip(row, r.label, r.tip);
    list.append(row);
  }
  host.replaceChildren(list);
}

/** @param {Stats} s */
function drawBands(s) {
  const total = s.players.playerDays;
  drawHBars(
    byId("chart-bands"),
    "Player-days by age band, last 30 days",
    BANDS.map((b) => ({
      label: BAND_LABEL[b] ?? b,
      value: s.players.playerDaysByBand[b],
      cls: `fill-${b}`,
      note: pct(s.players.playerDaysByBand[b], total),
      tip: [
        ["Player-days", num(s.players.playerDaysByBand[b])],
        ["Today", num(s.players.todayByBand[b])],
      ],
    })),
  );
}

/** @param {Stats} s */
function drawDurations(s) {
  const total = s.durations.reduce((a, d) => a + d.players, 0);
  drawHBars(
    byId("chart-durations"),
    "Longest session per player-day, last 30 days",
    s.durations.map((d) => ({
      label: DURATION_LABEL[d.bucket] ?? d.bucket,
      value: d.players,
      cls: "fill-bar",
      note: pct(d.players, total),
      tip: [["Player-days", num(d.players)]],
    })),
  );
}

/** @param {Stats} s @param {string} band */
function drawFunnel(s, band) {
  /** @param {BandCountsAll} c */
  const pick = (c) => (band === "all" ? c.all : c[/** @type {keyof BandCounts} */ (band)]);
  const started = pick(s.funnel.started);
  // The 3D game's quests first; the classic game's (/classic/) rooms and
  // trial only when that band has any.
  const classic = [
    ...s.funnel.rooms.map((r) => ({ label: `Classic room ${r.room + 1} clear`, value: pick(r) })),
    { label: "Classic trial finished", value: pick(s.funnel.trialComplete) },
  ].filter((st) => st.value > 0);
  const steps = [
    { label: "Started", value: started },
    ...s.funnel.quests.map((q) => ({ label: `Quest ${q.quest + 1} complete`, value: pick(q) })),
    ...classic,
  ];
  const cls = band === "all" ? "fill-bar" : `fill-${band}`;
  drawHBars(
    byId("chart-funnel"),
    `Funnel from session start to quests completed, ${band === "all" ? "all bands" : BAND_LABEL[band]}`,
    steps.map((st, i) => ({
      label: st.label,
      value: st.value,
      cls,
      note: i === 0 ? "" : pct(st.value, started),
      tip: [
        ["Player-days", num(st.value)],
        ["Of started", pct(st.value, started)],
      ],
    })),
  );
}

/** @param {Stats} s */
function fillTable(s) {
  const body = /** @type {HTMLTableElement} */ (byId("table-daily")).tBodies[0];
  if (!body) return;
  body.replaceChildren(
    ...[...s.daily].reverse().map((d) => {
      const tr = document.createElement("tr");
      for (const v of [d.day, num(d.players), num(d.sessions), ...BANDS.map((b) => num(d.byBand[b]))]) {
        const td = document.createElement("td");
        td.textContent = v;
        tr.append(td);
      }
      return tr;
    }),
  );
}

/** @param {Stats} s */
function render(s) {
  byId("kpi-today").textContent = num(s.players.today);
  byId("kpi-today-sub").textContent = BANDS.map((b) => `${(BAND_LABEL[b] ?? b).split(" ")[0]} ${num(s.players.todayByBand[b])}`).join(" · ");
  byId("kpi-sessions").textContent = num(s.sessions.today);
  byId("kpi-sessions-sub").textContent = `${num(s.sessions.total)} in 30 days`;
  byId("kpi-playerdays").textContent = num(s.players.playerDays);
  const quests = s.funnel.quests.reduce((a, q) => a + q.all, 0);
  byId("kpi-quests").textContent = num(quests);
  const first = s.funnel.quests[0]?.all ?? 0;
  byId("kpi-quests-sub").textContent = `quest 1: ${pct(first, s.funnel.started.all)} of started`;
  byId("kpi-trials").textContent = num(s.funnel.trialComplete.all);
  byId("kpi-trials-sub").textContent = `${pct(s.funnel.trialComplete.all, s.funnel.started.all)} of started`;
  drawDaily(s);
  drawBands(s);
  drawDurations(s);
  const select = /** @type {HTMLSelectElement} */ (byId("funnel-band"));
  drawFunnel(s, select.value);
  select.onchange = () => drawFunnel(s, select.value);
  fillTable(s);
  let width = byId("chart-daily").clientWidth;
  window.onresize = () => {
    const now = byId("chart-daily").clientWidth;
    if (now !== width) {
      width = now;
      drawDaily(s);
    }
  };
  byId("footnote").textContent =
    `Updated ${new Date(s.generatedAt).toLocaleString()}. Players are counted by a random id each device replaces every UTC day, ` +
    `so "player-days" add up days, not people. Raw events are deleted after ${s.retentionDays} days.`;
}

// -------------------------------------------------------------------- flow

function showLogin() {
  byId("login").hidden = false;
  byId("report").hidden = true;
  byId("actions").hidden = true;
  byId("token").focus();
}

async function load() {
  const token = getToken();
  if (!token) {
    showLogin();
    return;
  }
  setStatus("Loading…");
  /** @type {Response} */
  let res;
  try {
    res = await fetch("/api/stats", { headers: { Authorization: `Bearer ${token}` }, cache: "no-store", credentials: "omit" });
  } catch {
    setStatus("Could not reach /api/stats. Check your connection and try Refresh.", true);
    return;
  }
  if (res.status === 401) {
    setToken(null);
    showLogin();
    setStatus("That token was not accepted. Paste it again.", true);
    return;
  }
  if (!res.ok) {
    /** @type {{ error?: string }} */
    let body = {};
    try {
      body = await res.json();
    } catch {
      // not JSON (for example a 404 page when Functions are not deployed)
    }
    setStatus(`The stats endpoint answered ${res.status}${body.error ? `: ${body.error}` : ""}.`, true);
    return;
  }
  const stats = /** @type {Stats} */ (await res.json());
  byId("login").hidden = true;
  byId("report").hidden = false;
  byId("actions").hidden = false;
  render(stats);
  setStatus("");
}

byId("login").addEventListener("submit", (e) => {
  e.preventDefault();
  const input = /** @type {HTMLInputElement} */ (byId("token"));
  const value = input.value.trim();
  if (!value) return;
  setToken(value);
  input.value = "";
  void load();
});
byId("refresh").addEventListener("click", () => void load());
byId("forget").addEventListener("click", () => {
  setToken(null);
  setStatus("Token forgotten for this tab.");
  showLogin();
});

void load();

export {};
