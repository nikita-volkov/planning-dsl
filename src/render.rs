//! Renderers: derived artifacts — a mermaid flowchart of the task dependency
//! DAG, a self-contained interactive HTML graph, a mermaid gantt chart of the
//! schedule, and a self-contained interactive HTML gantt. All are pure
//! functions: same input in, byte-identical file out. The flowchart is a
//! function of the plan; the gantt is a function of the schedule artifact
//! alone; the HTML graph is a function of the plan plus the artifact's
//! critical path when handed one (`specs/scheduler.md`: renderers consume
//! it, nothing recomputes it); the HTML gantt is a function of the plan and
//! the artifact together — the artifact alone fixes every date, the plan is
//! read only for the dependency arrows and the authored fields the view
//! shows (role, estimate, unit) — and none re-derives a date, a capacity or
//! an assignment. The determinism is carried by the model's declaration
//! order (the `DeclarationMap`) and by canonicalizing the edge set, never by
//! hashing or sorting at the end.
//!
//! Renderers assume a valid input (the CLI gates on `validate`, and the
//! gantt's CLI path schedules first); defensively, a `depends-on` naming no
//! task is skipped, never rendered as a ghost node.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Duration};

use crate::model::{Plan, Status, Task};
use crate::schedule::{Schedule, TaskSchedule};

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

/// The interactive HTML gantt: one self-contained file, the schedule artifact
/// drawn as a horizontal timeline — a task-list column on the left, a day grid
/// on the right, a two-tier time axis (a band per calendar month over the
/// day-of-month numbers), gridlines at every day boundary. Rows group by
/// executor, first appearance along the artifact's task declaration order —
/// the mermaid gantt's precedent — with a full-width header band per executor;
/// within a group, declaration order. Done tasks carry no dates in the
/// artifact and are off the chart entirely. The dates come from the artifact
/// alone; the plan is read only for the dependency arrows (`edges`,
/// canonicalized — the DAG as authored, nothing recomputed) and for the
/// authored fields the detail card shows (role, estimate, unit). Bars carry
/// the status fill and the artifact's red stroke on critical-path members.
/// All geometry is computed here, in integers, so the bytes cannot depend on
/// float formatting; the inline ES5 script only reads what was printed
/// (data-* attributes): hovering a bar dims the chart and highlights the bar,
/// its direct dependency arrows and their other endpoints, click pins (the
/// same drag-vs-click rule as the HTML graph), a bottom-left card shows the
/// task's fields, and pan / wheel zoom / fit match the HTML graph's grammar.
/// The today line is drawn by the viewer's browser from the viewer's clock —
/// the file's bytes stay deterministic. No libs, no CDN, opens offline.
pub fn gantt_html(plan: &Plan, schedule: &Schedule) -> String {
    // Geometry, in integers, so the bytes cannot depend on float formatting.
    const DAY_W: usize = 26; // one day column
    const ROW_H: usize = 36;
    const BAR_H: usize = 20;
    const AXIS_H: usize = 44; // two tiers: the month band over the day numbers
    const CHAR_W: usize = 8; // 11px monospace advances ~6.6px; slack kept
    const PAD_X: usize = 10;
    const MIN_LABEL_W: usize = 120;
    const STUB: usize = 6; // the horizontal stub off a bar's right edge

    // The window: the artifact's span; with nothing left to do, its single
    // first day.
    let window_start = schedule.start;
    let window_end = schedule.plan_end.unwrap_or(schedule.start);
    let days = window_end
        .signed_duration_since(window_start)
        .num_days()
        .max(0) as usize
        + 1;

    // The dated lines, declaration order. Done tasks carry no dates — off the
    // chart entirely, the mermaid gantt's recorded precedent.
    let dated: Vec<(&str, &TaskSchedule)> = schedule
        .tasks
        .iter()
        .filter(|(_, line)| line.start.is_some() && line.end.is_some())
        .map(|(name, line)| (name.as_str(), line))
        .collect();

    // Executors in first-appearance order along that declaration order. A
    // dated task the artifact leaves unassigned (the scheduler never produces
    // one) renders, defensively, in an unnamed leading group with no header.
    let mut groups: Vec<Option<&str>> = Vec::new();
    for &(_, line) in &dated {
        let executor = line.executor.as_deref();
        if !groups.contains(&executor) {
            groups.push(executor);
        }
    }
    groups.sort_by_key(|g| g.is_some());

    // Rows: a header band per named group, one row per dated task; row
    // indices count bands too, everything hangs off the axis.
    let mut bands: Vec<(usize, &str)> = Vec::new();
    let mut geom: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
    let mut row = 0;
    for group in &groups {
        if let Some(name) = group {
            bands.push((row, name));
            row += 1;
        }
        for &(task, line) in &dated {
            if line.executor.as_deref() != *group {
                continue;
            }
            let start = line.start.expect("dated");
            let start_idx = start
                .signed_duration_since(window_start)
                .num_days()
                .max(0) as usize;
            let span = line
                .end
                .expect("dated")
                .signed_duration_since(start)
                .num_days()
                .max(0) as usize
                + 1;
            geom.insert(task, (start_idx, span, row));
            row += 1;
        }
    }
    let total_rows = row;

    // The label column fits the widest task name it carries.
    let label_w = dated
        .iter()
        .map(|(name, _)| name.chars().count() * CHAR_W + 2 * PAD_X)
        .max()
        .unwrap_or(MIN_LABEL_W)
        .max(MIN_LABEL_W);
    let canvas_w = label_w + days.saturating_mul(DAY_W);
    let canvas_h = AXIS_H + total_rows.saturating_mul(ROW_H);

    // The crit chain, read off the artifact: bars named in it wear the red
    // stroke, and an arrow wears it exactly when it is a consecutive pair of
    // the chain — dependency-first, so a pair runs dependency → dependent.
    let crit_nodes: BTreeSet<&str> = schedule.critical_path.iter().map(String::as_str).collect();
    let crit_edges: BTreeSet<(&str, &str)> = schedule
        .critical_path
        .windows(2)
        .map(|pair| (pair[0].as_str(), pair[1].as_str()))
        .collect();

    // The month tier: one label per calendar month spanning its days.
    let mut months: Vec<(usize, usize, String)> = Vec::new();
    for i in 0..days {
        let label = format!(
            "{:04}-{:02}",
            (window_start + Duration::days(i as i64)).year(),
            (window_start + Duration::days(i as i64)).month(),
        );
        match months.last_mut() {
            Some((_, last, same)) if *same == label => *last = i,
            _ => months.push((i, i, label)),
        }
    }

    let bar_y = |row: usize| AXIS_H + row * ROW_H + (ROW_H - BAR_H) / 2;
    let mid_y = |row: usize| AXIS_H + row * ROW_H + ROW_H / 2;
    let status_of = |line: &TaskSchedule| match line.status {
        Status::Todo => "todo",
        Status::InProgress => "in-progress",
        Status::Blocked => "blocked",
        Status::Done => "done",
    };

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{cw}\" height=\"{ch}\" \
         data-w=\"{cw}\" data-h=\"{ch}\" data-days=\"{days}\" data-day-w=\"{DAY_W}\" \
         data-label-w=\"{lw}\" data-start=\"{ws}\" data-end=\"{we}\">\n",
        cw = canvas_w,
        ch = canvas_h,
        lw = label_w,
        ws = window_start,
        we = window_end,
    ));
    svg.push_str(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
         <path d=\"M0,0 L10,5 L0,10 z\" fill=\"#5b6470\"/></marker>\
         <marker id=\"arrow-crit\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
         <path d=\"M0,0 L10,5 L0,10 z\" fill=\"#b91c1c\"/></marker></defs>\n",
    );
    svg.push_str("<g id=\"view\">\n");

    // The task-list column's panel, then the day grid: a line at every day
    // boundary, slightly darker where a month begins.
    svg.push_str(&format!(
        "<rect class=\"panel\" x=\"0\" y=\"0\" width=\"{lw}\" height=\"{ch}\"/>\n",
        lw = label_w,
        ch = canvas_h,
    ));
    for i in 0..days {
        let x = label_w + i * DAY_W;
        let class = if i > 0 && (window_start + Duration::days(i as i64)).day() == 1 {
            "grid month"
        } else {
            "grid"
        };
        svg.push_str(&format!(
            "<line class=\"{class}\" x1=\"{x}\" y1=\"0\" x2=\"{x}\" y2=\"{ch}\"/>\n",
            ch = canvas_h,
        ));
    }
    svg.push_str(&format!(
        "<line class=\"axis\" x1=\"0\" y1=\"{AXIS_H}\" x2=\"{cw}\" y2=\"{AXIS_H}\"/>\n",
        cw = canvas_w,
    ));

    // The time axis: a band label per calendar month, a number per day.
    for (first, last, label) in &months {
        let x = label_w + (first + last + 1) * DAY_W / 2;
        svg.push_str(&format!("<text class=\"month\" x=\"{x}\" y=\"11\">{label}</text>\n"));
    }
    for i in 0..days {
        let d = window_start + Duration::days(i as i64);
        let x = label_w + i * DAY_W + DAY_W / 2;
        svg.push_str(&format!(
            "<text class=\"day\" x=\"{x}\" y=\"33\">{}</text>\n",
            d.day()
        ));
    }

    // Full-width header bands with the executor name, then the task names in
    // the label column; the divider is repainted over both.
    for &(band_row, name) in &bands {
        let y = AXIS_H + band_row * ROW_H;
        svg.push_str(&format!(
            "<rect class=\"band\" x=\"0\" y=\"{y}\" width=\"{cw}\" height=\"{ROW_H}\"/>\n",
            cw = canvas_w,
        ));
        svg.push_str(&format!(
            "<text class=\"band-label\" x=\"{PAD_X}\" y=\"{}\">{}</text>\n",
            AXIS_H + band_row * ROW_H + ROW_H / 2,
            escape(name)
        ));
    }
    for group in &groups {
        for &(task, line) in &dated {
            if line.executor.as_deref() != *group {
                continue;
            }
            let task_row = geom[task].2;
            svg.push_str(&format!(
                "<text class=\"tlabel\" x=\"{PAD_X}\" y=\"{}\">{}</text>\n",
                mid_y(task_row),
                escape(task)
            ));
        }
    }
    svg.push_str(&format!(
        "<line class=\"divider\" x1=\"{lw}\" y1=\"0\" x2=\"{lw}\" y2=\"{ch}\"/>\n",
        lw = label_w,
        ch = canvas_h,
    ));

    // Arrows follow execution order, the community convention: they leave
    // the dependency's right edge and enter the dependent's left edge, both
    // at the bar's vertical middle — an orthogonal elbow: horizontal stub,
    // vertical travel, horizontal in. Saturating arithmetic guards the elbow
    // against a successor that does not start after its predecessor ends
    // (the scheduler guarantees it; the renderer is defensive). An endpoint
    // with no bar — a done task — takes no arrow.
    for (from, to) in edges(plan) {
        let (f, t) = match (geom.get(from), geom.get(to)) {
            (Some(f), Some(t)) => (*f, *t),
            _ => continue,
        };
        let sx = label_w
            .saturating_add(f.0.saturating_add(f.1).saturating_mul(DAY_W));
        let sy = mid_y(f.2);
        let ex = label_w.saturating_add(t.0.saturating_mul(DAY_W));
        let ey = mid_y(t.2);
        let mid = sx.saturating_add(STUB).min(ex).max(sx);
        let class = if crit_edges.contains(&(from, to)) {
            "link crit"
        } else {
            "link"
        };
        svg.push_str(&format!(
            "<path class=\"{class}\" data-from=\"{f}\" data-to=\"{t}\" \
             d=\"M {sx},{sy} H {mid} V {ey} H {ex}\"/>\n",
            f = escape(from),
            t = escape(to),
        ));
    }

    // Bars paint over the arrows. The card reads the task's fields off the
    // data attributes, so every field the card shows is printed here —
    // empty ones omitted (no executor, no estimate).
    for group in &groups {
        for &(task, line) in &dated {
            if line.executor.as_deref() != *group {
                continue;
            }
            let (start_idx, span, task_row) = geom[task];
            let x = label_w.saturating_add(start_idx.saturating_mul(DAY_W));
            let y = bar_y(task_row);
            let w = span.saturating_mul(DAY_W);
            let status = status_of(line);
            let crit = crit_nodes.contains(task);
            let class = if crit {
                format!("bar {status} crit")
            } else {
                format!("bar {status}")
            };
            let mut attrs = format!("data-id=\"{}\" data-status=\"{status}\"", escape(task));
            if let Some(executor) = line.executor.as_deref() {
                attrs.push_str(&format!(" data-executor=\"{}\"", escape(executor)));
            }
            attrs.push_str(&format!(
                " data-start=\"{}\" data-end=\"{}\"",
                line.start.expect("dated"),
                line.end.expect("dated"),
            ));
            if let Some(active) = line.active_days {
                attrs.push_str(&format!(" data-active-days=\"{active}\""));
            }
            let authored = &plan.tasks[task];
            attrs.push_str(&format!(" data-role=\"{}\"", escape(&authored.role)));
            if let Some(estimate) = authored.estimate {
                attrs.push_str(&format!(" data-estimate=\"{estimate}\""));
            }
            if crit {
                attrs.push_str(" data-crit=\"1\"");
            }
            svg.push_str(&format!(
                "<g class=\"{class}\" {attrs}>\
                 <rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{BAR_H}\" rx=\"4\"/></g>\n"
            ));
        }
    }
    svg.push_str("</g>\n</svg>\n");

    // The corner legend, one div per line so a future key joins as one more
    // push: the unit, always; the key to the red stroke, when a chain was
    // rendered; the keys to the status fills, when such a bar exists; the
    // today key, pre-printed hidden — the viewer's clock un-hides it.
    let mut legend = format!(
        "<div id=\"legend\">\n<div>unit: {}</div>\n",
        escape(&plan.unit)
    );
    if !schedule.critical_path.is_empty() {
        legend.push_str("<div>red stroke: critical path</div>\n");
    }
    if dated
        .iter()
        .any(|(_, line)| line.status == Status::InProgress)
    {
        legend.push_str("<div>blue: in-progress</div>\n");
    }
    if dated.iter().any(|(_, line)| line.status == Status::Blocked) {
        legend.push_str("<div>orange: blocked</div>\n");
    }
    legend.push_str("<div id=\"today-key\" hidden>dashed red: today</div>\n");
    legend.push_str("</div>\n");

    let mut out = String::from(GANTT_HEAD);
    out.push_str(&legend);
    out.push_str("<div id=\"card\"></div>\n");
    out.push_str(&svg);
    out.push_str(GANTT_TAIL);
    out
}
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
///
/// The schedule artifact, when handed one, marks the critical path: nodes
/// named in the artifact's `critical-path` chain wear `crit`, and an edge
/// wears `crit` exactly when it is a consecutive pair of the chain — the
/// chain is emitted dependency-first and the scheduler steps only along
/// real dependencies, so consecutive pairs are real edges by construction;
/// no heuristic. `None` marks nothing. Done tasks are never on the chain
/// (the scheduler leaves them off), so done muting and `crit` never
/// collide.
pub fn html(plan: &Plan, direction: Direction, schedule: Option<&Schedule>) -> String {
    let tasks = &plan.tasks;
    let edges = edges(plan);

    // The critical backbone, read off the artifact: the chain's nodes, and
    // its consecutive pairs as the crit edges (the chain runs
    // dependency-first, so a pair runs dependency → dependent).
    let crit_nodes: BTreeSet<&str> = match schedule {
        Some(s) => s.critical_path.iter().map(String::as_str).collect(),
        None => BTreeSet::new(),
    };
    let crit_edges: BTreeSet<(&str, &str)> = match schedule {
        Some(s) => s
            .critical_path
            .windows(2)
            .map(|pair| (pair[0].as_str(), pair[1].as_str()))
            .collect(),
        None => BTreeSet::new(),
    };

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
        let class = if crit_edges.contains(&(from, to)) {
            "edge crit"
        } else {
            "edge"
        };
        svg.push_str(&format!(
            "<path class=\"{class}\" data-from=\"{f}\" data-to=\"{t}\" marker-end=\"url(#arrow)\" \
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
        // Done is never on the chain (the scheduler leaves it off), so the
        // two branches cannot collide.
        let class = if crit_nodes.contains(name.as_str()) {
            "node crit"
        } else if done(task) {
            "node done"
        } else {
            "node"
        };
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
    // anything is muted; the key to the red backbone, when a chain was
    // rendered.
    let mut legend = format!(
        "<div id=\"legend\">\n<div>unit: {}</div>\n",
        escape(&plan.unit)
    );
    if tasks.values().any(|t| done(t)) {
        legend.push_str("<div>grey: done</div>\n");
    }
    if schedule.is_some_and(|s| !s.critical_path.is_empty()) {
        legend.push_str("<div>red: critical path</div>\n");
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
.node.crit rect { stroke: #b91c1c; }
.node.crit text { fill: #b91c1c; }
.edge.crit { stroke: #b91c1c; }
svg.dim .node, svg.dim .edge { opacity: 0.15; }
svg.dim .node.crit, svg.dim .edge.crit { opacity: 0.4; }
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

/// The head of the HTML gantt page: the same background and monospace family
/// as the HTML graph, its legend and dim/hot grammar, plus the gantt's own
/// panel, bands, axis, bars, links and the bottom-left detail card. Separate
/// from `HTML_HEAD` so the two files' styling cannot drift into each other.
const GANTT_HEAD: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>schedule gantt</title>
<style>
html, body { margin: 0; height: 100%; background: #fafaf7; }
body { font-family: ui-monospace, Menlo, Consolas, monospace; overflow: hidden; }
svg { width: 100vw; height: 100vh; display: block; cursor: grab; }
svg:active { cursor: grabbing; }
#legend { position: fixed; top: 12px; right: 16px; padding: 6px 10px; font-size: 11px; line-height: 1.7; color: #5b6470; background: rgba(255,255,255,0.9); border: 1px solid #c4cad2; border-radius: 6px; pointer-events: none; }
#card { position: fixed; left: 16px; bottom: 16px; display: none; padding: 8px 12px; font-size: 11px; line-height: 1.7; color: #1c2430; background: rgba(255,255,255,0.95); border: 1px solid #c4cad2; border-radius: 6px; pointer-events: none; }
#card .title { font-weight: 700; }
#card .crit { color: #b91c1c; }
.panel { fill: #f1f2ee; }
.band { fill: #eceee6; }
.grid { stroke: #e7e7df; }
.grid.month { stroke: #c4cad2; }
.axis, .divider { stroke: #c4cad2; }
.month { fill: #5b6470; font-size: 10px; text-anchor: middle; dominant-baseline: central; }
.day { fill: #9aa4b2; font-size: 9px; text-anchor: middle; dominant-baseline: central; }
.band-label { fill: #5b6470; font-size: 11px; font-weight: 700; dominant-baseline: central; }
.tlabel { fill: #1c2430; font-size: 11px; dominant-baseline: central; }
.bar { cursor: pointer; }
.bar.todo { fill: #5b6470; }
.bar.in-progress { fill: #2563eb; }
.bar.blocked { fill: #ea580c; }
.bar.done { fill: #c4cad2; }
.bar.crit { stroke: #b91c1c; stroke-width: 2; }
.link { stroke: #9aa4b2; stroke-width: 1.5; fill: none; marker-end: url(#arrow); }
.link.crit { stroke: #b91c1c; marker-end: url(#arrow-crit); }
svg.dim .bar, svg.dim .link { opacity: 0.15; }
svg.dim .bar.crit { opacity: 0.4; }
svg.dim .bar.hot, svg.dim .link.hot { opacity: 1; }
svg.dim .bar.hot { stroke: #b45309; stroke-width: 2; }
svg.dim .link.hot { stroke: #b45309; }
#today { stroke: #b91c1c; stroke-dasharray: 4 3; }
</style>
</head>
<body>
"#;

/// The tail of the HTML gantt page: hover dim + highlight of a bar, its
/// direct dependency arrows and their other endpoints, click-to-pin with the
/// four-pixel drag-vs-click rule, the bottom-left detail card built from the
/// bars' data attributes, pan, wheel zoom, fit — the HTML graph's script
/// grammar — plus the today line, drawn by the viewer's browser from the
/// viewer's clock (local date, never `toISOString`, which shifts by UTC) and
/// only inside the window, so the file's bytes stay deterministic.
const GANTT_TAIL: &str = r#"<script>
(function () {
  var svg = document.querySelector('svg');
  var view = document.getElementById('view');
  var W = +svg.dataset.w, H = +svg.dataset.h;
  var DAYS = +svg.dataset.days, DAY_W = +svg.dataset.dayW, LABEL_W = +svg.dataset.labelW;
  var bars = Array.prototype.slice.call(svg.querySelectorAll('.bar'));
  var links = Array.prototype.slice.call(svg.querySelectorAll('.link'));
  var byId = {};
  bars.forEach(function (b) { byId[b.getAttribute('data-id')] = b; });
  var tx = 0, ty = 0, k = 1;
  var pinned = null, down = null;
  var card = document.getElementById('card');

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
  function cardFor(bar) {
    var d = bar.dataset;
    while (card.firstChild) card.removeChild(card.firstChild);
    var add = function (text, cls) {
      var div = document.createElement('div');
      if (cls) div.className = cls;
      div.textContent = text;
      card.appendChild(div);
    };
    add(d.id, 'title');
    add('status: ' + d.status);
    if (d.executor) add('executor: ' + d.executor);
    add('start: ' + d.start);
    add('end: ' + d.end);
    if (d.activeDays) add('active-days: ' + d.activeDays);
    add(d.estimate ? d.role + ' · ' + d.estimate : d.role);
    if (d.crit) add('critical path', 'crit');
    card.style.display = 'block';
  }
  function setHot(id) {
    svg.classList.add('dim');
    bars.forEach(function (b) {
      if (b.getAttribute('data-id') === id) b.classList.add('hot');
    });
    links.forEach(function (l) {
      var f = l.getAttribute('data-from'), t = l.getAttribute('data-to');
      if (f !== id && t !== id) return;
      l.classList.add('hot');
      var other = f === id ? t : f;
      if (byId[other]) byId[other].classList.add('hot');
    });
    if (byId[id]) cardFor(byId[id]);
  }
  function clearHot() {
    svg.classList.remove('dim');
    bars.forEach(function (b) { b.classList.remove('hot'); });
    links.forEach(function (l) { l.classList.remove('hot'); });
    card.style.display = 'none';
  }
  // The pin: a click leaves a bar's highlight standing after mouseleave.
  // Hovering overrides it while over a bar; leaving restores it.
  function applyPin() {
    clearHot();
    if (pinned !== null) setHot(pinned);
  }

  bars.forEach(function (bar) {
    var id = bar.getAttribute('data-id');
    bar.addEventListener('mouseenter', function () { setHot(id); });
    bar.addEventListener('mouseleave', applyPin);
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
    // svg; the bar a click means is read off the down's target.
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
    // drag never moves the pin. Background (any non-bar) clears it.
    if (down && ev.button === 0) {
      var dx = ev.clientX - down.x, dy = ev.clientY - down.y;
      if (dx * dx + dy * dy <= 16) {
        var bar = down.target.closest('.bar');
        var id = bar ? bar.getAttribute('data-id') : null;
        pinned = id === pinned ? null : id;
        applyPin();
      }
    }
    drag = null;
    down = null;
  });
  svg.addEventListener('dblclick', fit);
  window.addEventListener('resize', fit);

  // The today line: the viewer's local date against the window Rust printed.
  var t = new Date();
  var utcDays = function (y, m, d) { return Date.UTC(y, m - 1, d) / 86400000; };
  var parts = svg.dataset.start.split('-');
  var idx = utcDays(t.getFullYear(), t.getMonth() + 1, t.getDate()) -
            utcDays(+parts[0], +parts[1], +parts[2]);
  if (idx >= 0 && idx < DAYS) {
    var x = LABEL_W + (idx + 0.5) * DAY_W;
    var line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    line.setAttribute('id', 'today');
    line.setAttribute('x1', x); line.setAttribute('x2', x);
    line.setAttribute('y1', 0); line.setAttribute('y2', H);
    view.appendChild(line);
    document.getElementById('today-key').hidden = false;
  }

  fit();
})();
</script>
</body>
</html>
"#;
