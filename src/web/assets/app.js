// skry dashboard. Read-only; renders the fleet state pushed over SSE.
// All remote-provided strings are inserted with textContent, never as HTML.
"use strict";

const $ = (id) => document.getElementById(id);
let fleet = { hosts: [], tls: [] };
let selected = null;
let view = "fleet";

function h(tag, attrs, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (k === "class") el.className = v;
    else if (k === "onclick") el.addEventListener("click", v);
    else if (k === "style") el.style.cssText = v;
    else el.setAttribute(k, v);
  }
  for (const c of children.flat()) {
    if (c === null || c === undefined || c === false) continue;
    el.append(c instanceof Node ? c : document.createTextNode(String(c)));
  }
  return el;
}

const STATUS_ORDER = { unreachable: 0, critical: 1, warning: 2, deviation: 3, ok: 4, pending: 5 };
const LEVEL_CLASS = { ok: "ok", warning: "warning", critical: "critical" };

function bytes(v) {
  if (v === null || v === undefined || !isFinite(v)) return "n/a";
  const u = ["B", "KiB", "MiB", "GiB", "TiB"];
  let i = 0;
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
  return (i === 0 || v >= 100 ? v.toFixed(0) : v.toFixed(1)) + " " + u[i];
}
const rate = (v) => (v === null || v === undefined ? "n/a" : bytes(v) + "/s");
const pct = (v) => (v === null || v === undefined ? "n/a" : v.toFixed(1) + "%");

function duration(s) {
  if (s === null || s === undefined) return "n/a";
  s = Math.floor(s);
  const d = Math.floor(s / 86400), hh = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
  if (d) return `${d}d ${hh}h`;
  if (hh) return `${hh}h ${m}m`;
  if (m) return `${m}m ${s % 60}s`;
  return `${s}s`;
}

const cpuPct = (m) => (m && m.cpu ? m.cpu.total_pct : null);
const memPct = (m) => (m && m.mem ? m.mem.used_pct : null);
const diskPct = (m) => (m && m.disks && m.disks.length ? Math.max(...m.disks.map((d) => d.used_pct)) : null);

function levelOf(v, warn, crit) {
  if (v === null || v === undefined) return "pending";
  return v >= crit ? "critical" : v >= warn ? "warning" : "ok";
}

function bar(v, warn, crit) {
  const w = v === null || v === undefined ? 0 : Math.max(0, Math.min(100, v));
  return h("div", { class: "bar" }, h("div", { class: "b-" + levelOf(v, warn, crit), style: `width:${w}%` }));
}

function probe(p, fn) {
  if (!p) return h("span", { class: "dim" }, "…");
  if (p.status === "ok") return fn(p.value);
  if (p.status === "unavailable") return h("span", { class: "dim" }, `n/a (${p.value})`);
  return h("span", { class: "dim" }, "…");
}

function visibleHosts() {
  const needle = $("filter").value.toLowerCase();
  const st = $("status").value;
  const sort = $("sort").value;
  const list = fleet.hosts.filter((x) => {
    if (st === "problems" && (x.status === "ok" || x.status === "pending")) return false;
    if (st === "unreachable" && x.status !== "unreachable") return false;
    if (!needle) return true;
    return x.name.toLowerCase().includes(needle) || (x.groups || []).some((g) => g.toLowerCase().includes(needle));
  });
  const metric = { cpu: cpuPct, memory: memPct, disk: diskPct }[sort];
  list.sort((a, b) => {
    if (sort === "status") return STATUS_ORDER[a.status] - STATUS_ORDER[b.status] || a.name.localeCompare(b.name);
    if (metric) return (metric(b.metrics) ?? -1) - (metric(a.metrics) ?? -1) || a.name.localeCompare(b.name);
    return a.name.localeCompare(b.name);
  });
  return list;
}

function renderCounts() {
  const counts = {};
  for (const x of fleet.hosts) counts[x.status] = (counts[x.status] || 0) + 1;
  const el = $("counts");
  el.replaceChildren(
    h("span", {}, `${fleet.hosts.length} hosts`),
    ...Object.keys(STATUS_ORDER).filter((s) => counts[s]).map((s) =>
      h("span", {}, h("span", { class: "dot b-" + s }), `${counts[s]} ${s}`))
  );
}

function tile(x) {
  const m = x.metrics;
  const failed = x.conn === "failed";
  const body = [];
  if (failed && x.error) {
    body.push(h("div", { class: "c-unreachable" }, "✖ " + x.error.kind.replace("_", " ")));
    body.push(h("div", { class: "err" }, x.error.message));
  } else if (!m) {
    body.push(h("div", { class: "dim" }, x.conn === "connecting" ? "connecting…" : "waiting…"));
  } else {
    const t = thresholds;
    body.push(h("div", { class: "row" }, "CPU", bar(cpuPct(m), t.cpu[0], t.cpu[1]), pct(cpuPct(m))));
    body.push(h("div", { class: "row" }, "MEM", bar(memPct(m), t.mem[0], t.mem[1]), pct(memPct(m))));
    body.push(h("div", { class: "row" }, "DSK", bar(diskPct(m), t.disk[0], t.disk[1]), pct(diskPct(m))));
    body.push(h("div", { class: "meta" },
      `load ${m.load ? m.load.one.toFixed(2) : "n/a"} · ↓${rate(m.net_rx_bps)} ↑${rate(m.net_tx_bps)}`));
    for (const d of x.deviations || []) body.push(h("div", { class: "flag c-deviation" }, `◆ ${d.metric} z=${d.z.toFixed(1)}`));
    if (x.findings && x.findings.length) body.push(h("div", { class: "flag c-warning" }, `⚑ ${x.findings.length} security finding(s)`));
  }
  return h("div", { class: "tile s-" + x.status, onclick: () => openDetail(x.name) },
    h("div", { class: "head" }, h("span", {}, x.name), h("span", { class: "st c-" + x.status }, x.status)),
    ...body,
    (x.groups || []).length ? h("div", { class: "meta" }, "@" + x.groups.join(" @")) : null);
}

// Default thresholds mirror the server defaults; the tile colours only.
const thresholds = { cpu: [80, 95], mem: [85, 95], disk: [80, 90] };

function renderFleet() {
  $("fleet").replaceChildren(...visibleHosts().map(tile));
}

function table(headers, rows) {
  return h("table", {}, h("tr", {}, ...headers.map((x) => h("th", {}, x))),
    ...rows.map((r) => h("tr", {}, ...r.map((c) => h("td", {}, c)))));
}

function renderSecurity() {
  const rows = fleet.hosts.map((x) => {
    const m = x.metrics;
    if (!m || x.conn === "failed") return [x.name, x.error ? x.error.kind : "pending", "", "", ""];
    const logins = m.failed_logins && m.failed_logins.status === "ok" ? String(m.failed_logins.value.total) : "n/a";
    const upd = m.updates && m.updates.status === "ok"
      ? `${m.updates.value.security ?? "?"} / ${m.updates.value.total}` : "n/a";
    const worst = (x.findings || []).reduce((a, f) => (f.level === "critical" || a === "critical" ? "critical" : f.level === "warning" ? "warning" : a), "ok");
    return [x.name, h("span", { class: "c-" + LEVEL_CLASS[worst] }, worst), logins, upd,
      (x.findings || []).map((f) => f.message).join("; ")];
  });
  const parts = [h("h3", {}, "Security pulse"),
    table(["Host", "Pulse", "Failed logins 24h", "Security / all updates", "Findings"], rows)];
  if (fleet.tls && fleet.tls.length) {
    parts.push(h("h3", {}, "TLS certificates"));
    parts.push(table(["Endpoint", "Status", "Days left", "Subject", "Issuer", "Problem"],
      fleet.tls.map((t) => [t.endpoint, h("span", { class: "c-" + LEVEL_CLASS[t.level] }, t.level),
        t.days_left ?? "n/a", t.subject || "", t.issuer || "", t.problem || ""])));
  }
  $("security").replaceChildren(...parts);
}

function openDetail(name) {
  selected = name;
  $("detail").hidden = false;
  renderDetail();
}

function renderDetail() {
  if (!selected) return;
  const x = fleet.hosts.find((y) => y.name === selected);
  if (!x) return;
  $("detail-title").replaceChildren(h("span", { class: "c-" + x.status }, "● "), x.name + " ", h("span", { class: "dim" }, x.status));
  const m = x.metrics;
  const parts = [];
  if (x.error) parts.push(h("div", { class: "err" }, `${x.error.kind}: ${x.error.message}`));
  if (!m) { $("detail-body").replaceChildren(...parts); return; }
  parts.push(h("div", { class: "kv" },
    "Address", x.addr || "", "Hostname", m.hostname, "OS", m.os || "", "Kernel", `${m.kernel} (${m.arch})`,
    "CPU", `${m.cores} cores${m.cpu_model ? " · " + m.cpu_model : ""}`, "Uptime", duration(m.uptime_secs),
    "IPs", (m.ips || []).filter((i) => i.scope !== "host").map((i) => i.cidr).join(", ")));
  const problems = [...(x.health.breaches || []).map((b) => `▲ ${b.metric} ${b.value.toFixed(1)} ≥ ${b.limit} (${b.level})`),
    ...(x.deviations || []).map((d) => `◆ ${d.metric} ${d.value.toFixed(1)} vs usual ${d.mean.toFixed(1)} (z=${d.z.toFixed(1)})`),
    ...(x.findings || []).map((f) => `⚑ ${f.message}`)];
  if (problems.length) {
    parts.push(h("h3", {}, "Problems"));
    parts.push(...problems.map((p) => h("div", {}, p)));
  }
  parts.push(h("h3", {}, "Resources"));
  const c = m.cpu;
  parts.push(h("div", { class: "kv" },
    "CPU", c ? `${c.total_pct.toFixed(1)}% (us ${c.user_pct.toFixed(0)} sy ${c.system_pct.toFixed(0)} io ${c.iowait_pct.toFixed(0)} st ${c.steal_pct.toFixed(0)})` : "n/a",
    "Memory", m.mem ? `${bytes(m.mem.used)} / ${bytes(m.mem.total)} (${m.mem.used_pct.toFixed(1)}%)` : "n/a",
    "Swap", m.mem ? `${bytes(m.mem.swap_used)} / ${bytes(m.mem.swap_total)}` : "n/a",
    "Load", m.load ? `${m.load.one.toFixed(2)} ${m.load.five.toFixed(2)} ${m.load.fifteen.toFixed(2)}` : "n/a",
    "Network", `↓ ${rate(m.net_rx_bps)} ↑ ${rate(m.net_tx_bps)}`));
  if (c && c.cores.length) {
    parts.push(h("div", { class: "cores" }, ...c.cores.map((v, i) =>
      h("div", {}, `cpu${i} `, bar(v, 80, 95), ` ${v.toFixed(0)}%`))));
  }
  parts.push(h("h3", {}, "Filesystems"));
  parts.push(table(["Mount", "Device", "Used", "Size", "Use"], (m.disks || []).map((d) =>
    [d.mount, d.filesystem, bytes(d.used), bytes(d.total), pct(d.used_pct)])));
  parts.push(h("h3", {}, "Disk I/O"));
  parts.push(table(["Device", "Read", "Write", "IOPS r/w", "Util"], (m.disk_io || []).map((d) =>
    [d.device, rate(d.read_bps), rate(d.write_bps), `${d.read_iops.toFixed(0)}/${d.write_iops.toFixed(0)}`, pct(d.util_pct)])));
  parts.push(h("h3", {}, "Network"));
  parts.push(table(["Interface", "RX", "TX", "Errors", "Drops"], (m.net || []).map((n) =>
    [n.iface, rate(n.rx_bps), rate(n.tx_bps), n.errors, n.drops])));
  parts.push(h("h3", {}, `Top processes (${m.proc_count} total)`));
  parts.push(table(["PID", "User", "Command", "CPU%", "Mem%", "RSS"], (m.procs || []).map((p) =>
    [p.pid, p.user, p.name, p.cpu_pct === null ? "…" : p.cpu_pct.toFixed(1), p.mem_pct.toFixed(1), bytes(p.rss)])));
  parts.push(h("h3", {}, "Containers"));
  parts.push(probe(m.containers, (list) => list.length
    ? table(["Name", "Image", "Status", "CPU", "Memory"], list.map((k) => [k.name, k.image, k.status, pct(k.cpu_pct), k.mem_usage || "n/a"]))
    : h("span", { class: "dim" }, "none")));
  parts.push(h("h3", {}, "Failed systemd units"));
  parts.push(probe(m.failed_units, (list) => list.length
    ? table(["Unit", "State", "Description"], list.map((u) => [u.unit, `${u.active}/${u.sub}`, u.description]))
    : h("span", { class: "c-ok" }, "none")));
  parts.push(h("h3", {}, "Security"));
  parts.push(h("div", { class: "kv" },
    "Listening", probe(m.ports, (p) => h("span", {}, p.map((q) => (q.addr.includes(":") ? `[${q.addr}]` : q.addr) + ":" + q.port).join(", "))),
    "Failed logins", probe(m.failed_logins, (l) => h("span", {}, `${l.total} in 24h (${l.source})` +
      (l.top_sources.length ? " · " + l.top_sources.map(([ip, n]) => `${ip} ×${n}`).join(", ") : ""))),
    "Updates", probe(m.updates, (u) => h("span", {}, `${u.total} pending, ${u.security ?? "unknown"} security (${u.manager})`))));
  $("detail-body").replaceChildren(...parts);
}

function render() {
  renderCounts();
  if (view === "fleet") renderFleet(); else renderSecurity();
  if (!$("detail").hidden) renderDetail();
}

function setView(v) {
  view = v;
  $("fleet").hidden = v !== "fleet";
  $("security").hidden = v !== "security";
  $("tab-fleet").classList.toggle("active", v === "fleet");
  $("tab-security").classList.toggle("active", v === "security");
  render();
}

function connect() {
  const es = new EventSource("/api/events");
  es.addEventListener("fleet", (ev) => {
    fleet = JSON.parse(ev.data);
    $("live").textContent = "LIVE";
    $("live").classList.add("on");
    render();
  });
  es.onerror = () => {
    $("live").textContent = "reconnecting…";
    $("live").classList.remove("on");
  };
}

for (const id of ["filter", "status", "sort"]) $(id).addEventListener("input", render);
$("tab-fleet").addEventListener("click", () => setView("fleet"));
$("tab-security").addEventListener("click", () => setView("security"));
$("detail-close").addEventListener("click", () => { $("detail").hidden = true; selected = null; });
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") { $("detail").hidden = true; selected = null; }
});
if (location.search.includes("token=")) history.replaceState(null, "", "/");
connect();
