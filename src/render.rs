//! Renderers: derived artifacts — a mermaid flowchart of the task dependency
//! DAG, a self-contained interactive HTML graph, and a mermaid gantt chart of
//! the schedule. All are pure functions: same input in, byte-identical file
//! out. The flowchart and HTML are functions of the plan; the gantt is a
//! function of the schedule artifact alone (`specs/scheduler.md`: renderers
//! consume it, nothing recomputes it) — it never re-derives a date, a
//! capacity or an assignment. The determinism is carried by the model's
//! declaration order (the `DeclarationMap`) and by canonicalizing the edge
//! set, never by hashing or sorting at the end.
//!
//! Renderers assume a valid input (the CLI gates on `validate`, and the
//! gantt's CLI path schedules first); defensively, a `depends-on` naming no
//! task is skipped, never rendered as a ghost node.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Plan, Status, Task};
use crate::schedule::Schedule;

/// The dependency edges, canonicalized: `(dependency, dependent)` pairs —
/// what must come first, what waits for it — sorted, deduplicated, and
/// stripped of dependencies that name no task. Arrows follow execution
/// order, the community convention.
pub fn edges(plan: &Plan) -> BTreeSet<(&str, &str)> {
    let mut edges = BTreeSet::new();
    for (name, task) in &plan.tasks {
        for dep in &task.depends_on {
            if plan.tasks.contains_key(dep) {
                edges.insert((dep.as_str(), name.as_str()));
            }
        }
    }
    edges
}

/// The mermaid flowchart: one node per task, one edge per dependency.
pub fn mermaid(plan: &Plan) -> String {
    let ids = node_ids(plan);
    let mut out = String::from("flowchart TD\n");
    for name in plan.tasks.keys() {
        out.push_str(&format!(
            "    {}[\"{}\"]\n",
            ids[name.as_str()],
            escape(name)
        ));
    }
    for (from, to) in edges(plan) {
        out.push_str(&format!("    {} --> {}\n", ids[from], ids[to]));
    }
    out
}

/// Node ids: the task names themselves when every one of them is a safe
/// mermaid identifier — the raw file stays readable — synthetic `t0…tN`
/// otherwise. Mermaid reserves words like `end` and chokes on names with
/// spaces, so the check is strict; one global decision, never a mix.
fn node_ids(plan: &Plan) -> BTreeMap<&str, String> {
    if plan.tasks.keys().all(|n| is_safe_mermaid_id(n)) {
        plan.tasks
            .keys()
            .map(|n| (n.as_str(), n.to_string()))
            .collect()
    } else {
        plan.tasks
            .keys()
            .enumerate()
            .map(|(i, n)| (n.as_str(), format!("t{i}")))
            .collect()
    }
}

fn is_safe_mermaid_id(name: &str) -> bool {
    const RESERVED: [&str; 11] = [
        "end",
        "o",
        "x",
        "graph",
        "flowchart",
        "subgraph",
        "direction",
        "classDef",
        "style",
        "click",
        "linkStyle",
    ];
    match name.chars().next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && !RESERVED.contains(&name)
}

/// Escape a label for a mermaid quoted string and for XML text/attributes at
/// once: the four characters that break either, written as entities both
/// mermaid and the browser render back.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The mermaid gantt chart, drawn from the schedule artifact and nothing
/// else: one bar per task carrying dates, grouped into `section`s per
/// assigned executor. The artifact does not carry the plan's executor
/// declaration order, so sections follow the executor's first appearance
/// along the artifact's task declaration order; within a section, tasks keep
/// declaration order. Done tasks carry no dates in the artifact — they take
/// no bar and no section. Critical-path tasks are tagged `crit`. There are
/// no `excludes` lines: the artifact carries no calendar, and inventing one
/// here would recompute the pauses the scheduler already folded into the
/// dates.
pub fn gantt(schedule: &Schedule) -> String {
    let mut out = String::from("gantt\n    dateFormat YYYY-MM-DD\n    axisFormat %Y-%m-%d\n");
    // Executors in first-appearance order; `None` (a dated task the artifact
    // leaves unassigned — the scheduler never produces one) draws its tasks
    // before any section, in mermaid's implicit first group.
    let mut executors: Vec<Option<&str>> = Vec::new();
    for line in schedule.tasks.values() {
        let executor = line.executor.as_deref();
        if !executors.contains(&executor) {
            executors.push(executor);
        }
    }
    for executor in executors {
        if let Some(name) = executor {
            out.push_str(&format!("    section {}\n", escape(name)));
        }
        for (task, line) in &schedule.tasks {
            if line.executor.as_deref() != executor {
                continue;
            }
            if let (Some(start), Some(end)) = (line.start, line.end) {
                let crit = if schedule.critical_path.iter().any(|c| c == task) {
                    "crit, "
                } else {
                    ""
                };
                out.push_str(&format!(
                    "    {} :{}{}, {}\n",
                    escape(task),
                    crit,
                    start,
                    end
                ));
            }
        }
    }
    out
}

/// The flow direction of the HTML graph's layout: `Tb` stacks one row per
/// layer, dependencies at the top and dependents below; `Lr` keeps the
/// original left-to-right columns. A property of the view, like `--gantt`
/// is a property of the output set — never of the plan
/// (`decisions/the-html-graph-renders-top-down-by-default.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Top-down: the default.
    Tb,
    /// Left-to-right.
    Lr,
}

/// The interactive HTML graph: one self-contained file. The SVG geometry is
/// computed here — one row or column per layer along the flow
/// (`Direction`), dependencies upstream of dependents — and the inline JS
/// adds hover highlighting (a task, its dependency edges and its
/// neighbours), click-to-pin, pan, wheel zoom and fit-on-load. Nodes carry
/// a second label line, `role · estimate`; done tasks are muted; a corner
/// legend states the estimate unit. No CDN, no network, no layout library;
/// it opens offline.
pub fn html(plan: &Plan, direction: Direction) -> String {
    let tasks = &plan.tasks;
    let edges = edges(plan);

    // Layer = longest path from a root, by bounded fixed-point iteration. A
    // plan that renders is acyclic so this converges; the bound keeps a
    // defensively-rendered cyclic one from hanging.
    let mut layer: BTreeMap<&str, usize> = tasks.keys().map(|k| (k.as_str(), 0)).collect();
    for _ in 0..tasks.len() {
        let mut changed = false;
        for (name, task) in tasks {
            let next = task
                .depends_on
                .iter()
                .filter(|d| tasks.contains_key(*d))
                .map(|d| layer[d.as_str()] + 1)
                .max()
                .unwrap_or(0);
            if next > layer[name.as_str()] {
                layer.insert(name.as_str(), next);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Geometry, in integers, so the bytes cannot depend on float
    // formatting. The layers flow along the direction: top-down, a row per
    // layer stacked vertically — dependencies at the top, dependents below;
    // left-to-right, a column per layer — dependencies on the left,
    // dependents on the right. Rows keep declaration order; each row is
    // centered across the flow.
    const ROW_H: usize = 56;
    const NODE_H: usize = 40; // two text lines: the name, and role · estimate
    const NAME_Y: usize = 13; // the 12px name line, centered in the top half
    const SUB_Y: usize = 28; // the 10px role·estimate line, in the bottom half
    const CHAR_W: usize = 8; // 12px monospace advances ~7.2px; slack kept
    const PAD_X: usize = 10;
    const MIN_W: usize = 100;
    const H_GAP: usize = 60;
    const MARGIN: usize = 24;

    // The second label line: `role · estimate`, the estimate written bare
    // ("6", "4.5"); without one, just the role. The unit is stated once, in
    // the corner legend.
    let sub_label = |t: &Task| match t.estimate {
        Some(e) => format!("{} · {}", t.role, e),
        None => t.role.clone(),
    };
    // The node's width follows the longer of its two label lines.
    let width = |name: &str, t: &Task| {
        let line_w = |s: &str| s.chars().count() * CHAR_W + 2 * PAD_X;
        line_w(name).max(line_w(&sub_label(t))).max(MIN_W)
    };

    let max_layer = layer.values().copied().max().unwrap_or(0);
    let mut rows: Vec<Vec<&str>> = vec![Vec::new(); max_layer + 1];
    for name in tasks.keys() {
        rows[layer[name.as_str()]].push(name.as_str());
    }
    let max_rows = rows.iter().map(Vec::len).max().unwrap_or(0);

    let mut pos: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let (canvas_w, canvas_h) = match direction {
        Direction::Lr => {
            let col_w = rows
                .iter()
                .flatten()
                .copied()
                .map(|n| width(n, &tasks[n]))
                .max()
                .unwrap_or(MIN_W)
                + H_GAP;
            for (l, col) in rows.iter().enumerate() {
                let top = MARGIN + (max_rows - col.len()) * ROW_H / 2;
                for (i, &name) in col.iter().enumerate() {
                    pos.insert(name, (MARGIN + l * col_w, top + i * ROW_H));
                }
            }
            (
                MARGIN + (max_layer + 1) * col_w,
                MARGIN * 2 + max_rows * ROW_H,
            )
        }
        Direction::Tb => {
            // The canvas width fits the widest layer; every row is centered
            // horizontally within it.
            let row_w = |col: &[&str]| -> usize {
                col.iter().map(|n| width(n, &tasks[*n])).sum::<usize>()
                    + col.len().saturating_sub(1) * H_GAP
            };
            let flow_w = rows.iter().map(|c| row_w(c)).max().unwrap_or(0);
            for (l, col) in rows.iter().enumerate() {
                let mut x = MARGIN + (flow_w - row_w(col)) / 2;
                let top = MARGIN + l * ROW_H;
                for &name in col {
                    pos.insert(name, (x, top));
                    x += width(name, &tasks[name]) + H_GAP;
                }
            }
            (MARGIN * 2 + flow_w, MARGIN * 2 + (max_layer + 1) * ROW_H)
        }
    };

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{cw}\" height=\"{ch}\" data-w=\"{cw}\" data-h=\"{ch}\">\n",
        cw = canvas_w,
        ch = canvas_h,
    ));
    svg.push_str(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
         <path d=\"M0,0 L10,5 L0,10 z\" fill=\"#5b6470\"/></marker></defs>\n",
    );
    svg.push_str("<g id=\"view\">\n");

    // Edges first, so nodes paint over them; an edge leaves the dependency
    // where the flow exits it and enters the dependent where the flow
    // arrives — bottom-center to top-center in tb, right edge to left edge
    // in lr — arrowhead at the dependent: arrows follow execution order.
    // The bezier's control points sit on the midpoint of the axis the flow
    // runs along.
    for &(from, to) in &edges {
        let (fx, fy) = pos[from];
        let (tx, ty) = pos[to];
        let d = match direction {
            Direction::Lr => {
                let (sx, sy) = (fx + width(from, &tasks[from]), fy + NODE_H / 2);
                let (ex, ey) = (tx, ty + NODE_H / 2);
                let mx = (sx + ex) / 2;
                format!("M {sx},{sy} C {mx},{sy} {mx},{ey} {ex},{ey}")
            }
            Direction::Tb => {
                let (sx, sy) = (fx + width(from, &tasks[from]) / 2, fy + NODE_H);
                let (ex, ey) = (tx + width(to, &tasks[to]) / 2, ty);
                let my = (sy + ey) / 2;
                format!("M {sx},{sy} C {sx},{my} {ex},{my} {ex},{ey}")
            }
        };
        svg.push_str(&format!(
            "<path class=\"edge\" data-from=\"{f}\" data-to=\"{t}\" marker-end=\"url(#arrow)\" \
             d=\"{d}\"/>\n",
            f = escape(from),
            t = escape(to),
        ));
    }
    let done = |t: &Task| t.status == Some(Status::Done);
    for name in tasks.keys() {
        let task = &tasks[name.as_str()];
        let (x, y) = pos[name.as_str()];
        let label = escape(name);
        let sub = escape(&sub_label(task));
        let class = if done(task) { "node done" } else { "node" };
        svg.push_str(&format!(
            "<g class=\"{class}\" data-id=\"{label}\" transform=\"translate({x},{y})\">\
             <rect width=\"{w}\" height=\"{nh}\" rx=\"6\"/>\
             <text x=\"{px}\" y=\"{ny}\">{label}</text>\
             <text class=\"sub\" x=\"{px}\" y=\"{sy}\">{sub}</text>\
             <title>{label}</title></g>\n",
            w = width(name, task),
            nh = NODE_H,
            px = PAD_X,
            ny = NAME_Y,
            sy = SUB_Y,
        ));
    }
    svg.push_str("</g>\n</svg>\n");

    // The corner legend, one div per line so a future key joins as one more
    // push: the estimate unit, always; the key to the muted look, when
    // anything is muted.
    let mut legend = format!(
        "<div id=\"legend\">\n<div>unit: {}</div>\n",
        escape(&plan.unit)
    );
    if tasks.values().any(|t| done(t)) {
        legend.push_str("<div>grey: done</div>\n");
    }
    legend.push_str("</div>\n");

    let mut out = String::from(HTML_HEAD);
    out.push_str(&legend);
    out.push_str(&svg);
    out.push_str(HTML_TAIL);
    out
}

const HTML_HEAD: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>task dependency graph</title>
<style>
html, body { margin: 0; height: 100%; background: #fafaf7; }
body { font-family: ui-monospace, Menlo, Consolas, monospace; overflow: hidden; }
svg { width: 100vw; height: 100vh; display: block; cursor: grab; }
svg:active { cursor: grabbing; }
#legend { position: fixed; top: 12px; right: 16px; padding: 6px 10px; font-size: 11px; line-height: 1.7; color: #5b6470; background: rgba(255,255,255,0.9); border: 1px solid #c4cad2; border-radius: 6px; pointer-events: none; }
.node { cursor: pointer; }
.node rect { fill: #ffffff; stroke: #5b6470; stroke-width: 1.5; }
.node text { fill: #1c2430; font-size: 12px; dominant-baseline: central; }
.node text.sub { fill: #9aa4b2; font-size: 10px; }
.node.done rect { fill: #f1f2ee; stroke: #c4cad2; }
.node.done text { fill: #9aa4b2; }
.edge { stroke: #9aa4b2; stroke-width: 1.5; fill: none; }
svg.dim .node, svg.dim .edge { opacity: 0.15; }
svg.dim .node.hot, svg.dim .edge.hot { opacity: 1; }
svg.dim .node.hot rect { fill: #fff3e0; stroke: #b45309; }
svg.dim .node.hot text { fill: #7c3a00; }
svg.dim .edge.hot { stroke: #b45309; }
</style>
</head>
<body>
"#;

const HTML_TAIL: &str = r#"<script>
(function () {
  var svg = document.querySelector('svg');
  var view = document.getElementById('view');
  var W = +svg.dataset.w, H = +svg.dataset.h;
  var nodes = Array.prototype.slice.call(svg.querySelectorAll('.node'));
  var edges = Array.prototype.slice.call(svg.querySelectorAll('.edge'));
  var tx = 0, ty = 0, k = 1;
  var pinned = null, down = null;

  function apply() {
    view.setAttribute('transform', 'translate(' + tx + ',' + ty + ') scale(' + k + ')');
  }
  function fit() {
    k = Math.min((window.innerWidth - 48) / W, (window.innerHeight - 48) / H, 1);
    if (!isFinite(k) || k <= 0) k = 1;
    tx = (window.innerWidth - W * k) / 2;
    ty = (window.innerHeight - H * k) / 2;
    apply();
  }
  function setHot(id) {
    svg.classList.add('dim');
    nodes.forEach(function (m) {
      if (m.getAttribute('data-id') === id) m.classList.add('hot');
    });
    edges.forEach(function (e) {
      var f = e.getAttribute('data-from'), t = e.getAttribute('data-to');
      if (f !== id && t !== id) return;
      e.classList.add('hot');
      var other = f === id ? t : f;
      nodes.forEach(function (m) {
        if (m.getAttribute('data-id') === other) m.classList.add('hot');
      });
    });
  }
  function clearHot() {
    svg.classList.remove('dim');
    nodes.forEach(function (m) { m.classList.remove('hot'); });
    edges.forEach(function (e) { e.classList.remove('hot'); });
  }
  // The pin: a click leaves a node's highlight standing after mouseleave.
  // Hovering overrides it while over a node; leaving restores it.
  function applyPin() {
    clearHot();
    if (pinned !== null) setHot(pinned);
  }

  nodes.forEach(function (node) {
    var id = node.getAttribute('data-id');
    node.addEventListener('mouseenter', function () { setHot(id); });
    node.addEventListener('mouseleave', applyPin);
  });

  svg.addEventListener('wheel', function (ev) {
    ev.preventDefault();
    var r = svg.getBoundingClientRect();
    var f = ev.deltaY < 0 ? 1.1 : 1 / 1.1;
    var mx = ev.clientX - r.left, my = ev.clientY - r.top;
    tx = mx - (mx - tx) * f;
    ty = my - (my - ty) * f;
    k *= f;
    apply();
  }, { passive: false });

  var drag = null;
  svg.addEventListener('pointerdown', function (ev) {
    if (ev.button !== 0) return;
    drag = { x: ev.clientX, y: ev.clientY, tx: tx, ty: ty };
    // The pointer is captured to the svg on down, so the up targets the
    // svg; the node a click means is read off the down's target.
    down = { x: ev.clientX, y: ev.clientY, target: ev.target };
    svg.setPointerCapture(ev.pointerId);
  });
  svg.addEventListener('pointermove', function (ev) {
    if (!drag) return;
    tx = drag.tx + ev.clientX - drag.x;
    ty = drag.ty + ev.clientY - drag.y;
    apply();
  });
  svg.addEventListener('pointerup', function (ev) {
    // A click is an up that stayed within a few pixels of its down; a pan
    // drag never moves the pin. Background (any non-node) clears it.
    if (down && ev.button === 0) {
      var dx = ev.clientX - down.x, dy = ev.clientY - down.y;
      if (dx * dx + dy * dy <= 16) {
        var node = down.target.closest('.node');
        var id = node ? node.getAttribute('data-id') : null;
        pinned = id === pinned ? null : id;
        applyPin();
      }
    }
    drag = null;
    down = null;
  });
  svg.addEventListener('dblclick', fit);
  window.addEventListener('resize', fit);
  fit();
})();
</script>
</body>
</html>
"#;
