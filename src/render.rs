//! Renderers: derived artifacts over the typed plan model — a mermaid
//! flowchart of the task dependency DAG and a self-contained interactive HTML
//! graph. Both are pure functions of the plan: same plan in, byte-identical
//! file out. The determinism is carried by the `BTreeMap` ordering of the
//! model and by canonicalizing the edge set, never by hashing or sorting at
//! the end.
//!
//! Renderers assume a valid plan (the CLI gates on `validate` first);
//! defensively, a `depends-on` naming no task is skipped, never rendered as
//! a ghost node.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::Plan;

/// The dependency edges, canonicalized: `(task, dependency)` pairs — a task
/// on the left, what it depends on on the right — sorted, deduplicated, and
/// stripped of dependencies that name no task.
pub fn edges(plan: &Plan) -> BTreeSet<(&str, &str)> {
    let mut edges = BTreeSet::new();
    for (name, task) in &plan.tasks {
        for dep in &task.depends_on {
            if plan.tasks.contains_key(dep) {
                edges.insert((name.as_str(), dep.as_str()));
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

/// The interactive HTML graph: one self-contained file — the SVG geometry is
/// computed here, the inline JS adds hover highlighting (a task, its
/// dependency edges and its neighbours), pan, wheel zoom and fit-on-load.
/// No CDN, no network, no layout library; it opens offline.
pub fn html(plan: &Plan) -> String {
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

    // Geometry, in integers, so the bytes cannot depend on float formatting:
    // a column per layer (dependencies on the left, dependents on the right),
    // rows ordered by task name, columns centered vertically.
    const ROW_H: usize = 44;
    const NODE_H: usize = 28;
    const CHAR_W: usize = 8; // 12px monospace advances ~7.2px; slack kept
    const PAD_X: usize = 10;
    const MIN_W: usize = 100;
    const H_GAP: usize = 60;
    const MARGIN: usize = 24;

    let width = |n: &str| (n.chars().count() * CHAR_W + 2 * PAD_X).max(MIN_W);
    let max_layer = layer.values().copied().max().unwrap_or(0);
    let mut rows: Vec<Vec<&str>> = vec![Vec::new(); max_layer + 1];
    for name in tasks.keys() {
        rows[layer[name.as_str()]].push(name.as_str());
    }
    let max_rows = rows.iter().map(Vec::len).max().unwrap_or(0);
    let col_w = rows
        .iter()
        .flatten()
        .map(|n| width(n))
        .max()
        .unwrap_or(MIN_W)
        + H_GAP;
    let canvas_w = MARGIN + (max_layer + 1) * col_w;
    let canvas_h = MARGIN * 2 + max_rows * ROW_H;

    let mut pos: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for (l, col) in rows.iter().enumerate() {
        let top = MARGIN + (max_rows - col.len()) * ROW_H / 2;
        for (i, &name) in col.iter().enumerate() {
            pos.insert(name, (MARGIN + l * col_w, top + i * ROW_H));
        }
    }

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

    // Edges first, so nodes paint over them; an edge leaves the dependent's
    // left edge and enters the dependency's right edge, arrowhead at the
    // dependency.
    for &(from, to) in &edges {
        let (fx, fy) = pos[from];
        let (tx, ty) = pos[to];
        let (sx, sy) = (fx, fy + NODE_H / 2);
        let (ex, ey) = (tx + width(to), ty + NODE_H / 2);
        let mx = (sx + ex) / 2;
        svg.push_str(&format!(
            "<path class=\"edge\" data-from=\"{f}\" data-to=\"{t}\" marker-end=\"url(#arrow)\" \
             d=\"M {sx},{sy} C {mx},{sy} {mx},{ey} {ex},{ey}\"/>\n",
            f = escape(from),
            t = escape(to),
        ));
    }
    for name in tasks.keys() {
        let (x, y) = pos[name.as_str()];
        let label = escape(name);
        svg.push_str(&format!(
            "<g class=\"node\" data-id=\"{label}\" transform=\"translate({x},{y})\">\
             <rect width=\"{w}\" height=\"{nh}\" rx=\"6\"/>\
             <text x=\"{px}\" y=\"{ny}\">{label}</text><title>{label}</title></g>\n",
            w = width(name),
            nh = NODE_H,
            px = PAD_X,
            ny = NODE_H / 2,
        ));
    }
    svg.push_str("</g>\n</svg>\n");

    let mut out = String::from(HTML_HEAD);
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
.node { cursor: pointer; }
.node rect { fill: #ffffff; stroke: #5b6470; stroke-width: 1.5; }
.node text { fill: #1c2430; font-size: 12px; dominant-baseline: central; }
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

  nodes.forEach(function (node) {
    var id = node.getAttribute('data-id');
    node.addEventListener('mouseenter', function () {
      svg.classList.add('dim');
      node.classList.add('hot');
      edges.forEach(function (e) {
        var f = e.getAttribute('data-from'), t = e.getAttribute('data-to');
        if (f !== id && t !== id) return;
        e.classList.add('hot');
        var other = f === id ? t : f;
        nodes.forEach(function (m) {
          if (m.getAttribute('data-id') === other) m.classList.add('hot');
        });
      });
    });
    node.addEventListener('mouseleave', function () {
      svg.classList.remove('dim');
      nodes.forEach(function (m) { m.classList.remove('hot'); });
      edges.forEach(function (e) { e.classList.remove('hot'); });
    });
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
    drag = { x: ev.clientX, y: ev.clientY, tx: tx, ty: ty };
    svg.setPointerCapture(ev.pointerId);
  });
  svg.addEventListener('pointermove', function (ev) {
    if (!drag) return;
    tx = drag.tx + ev.clientX - drag.x;
    ty = drag.ty + ev.clientY - drag.y;
    apply();
  });
  svg.addEventListener('pointerup', function () { drag = null; });
  svg.addEventListener('dblclick', fit);
  window.addEventListener('resize', fit);
  fit();
})();
</script>
</body>
</html>
"#;
