//! Renderers: derived artifacts — a mermaid flowchart of the task dependency
//! DAG, a self-contained interactive HTML graph, a mermaid gantt chart of the
//! schedule, and a self-contained interactive HTML gantt. All are pure
//! functions: same input in, byte-identical file out. The flowchart is a
//! function of the plan; the gantt is a function of the schedule artifact
//! alone; the HTML graph is a function of the plan plus the artifact when
//! handed one — its critical path and the per-task lines the detail card
//! reads (`specs/scheduler.md`: renderers consume it, nothing recomputes
//! it); the HTML gantt is a function of the plan and the artifact together
//! — the artifact alone fixes every date, the plan is read only for the
//! dependency arrows and the authored fields the view shows (role,
//! estimate, unit) — and none re-derives a date, a capacity or an
//! assignment. The determinism is carried by the model's declaration order
//! (the `DeclarationMap`) and by canonicalizing the edge set, never by
//! hashing or sorting at the end.
//!
//! Renderers assume a valid input (the CLI gates on `validate`, and the
//! gantt's CLI path schedules first); defensively, a `depends-on` naming no
//! task is skipped, never rendered as a ghost node.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Duration};
use serde_json::{json, Map as JsonMap, Value as JsonValue};

use crate::model::{Plan, Status};
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

/// The status in its kebab-case spelling — the same string serde's
/// `rename_all` writes into `plan.yaml` and the artifact.
fn status_label(status: Status) -> &'static str {
    match status {
        Status::Todo => "todo",
        Status::InProgress => "in-progress",
        Status::Blocked => "blocked",
        Status::Done => "done",
    }
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

/// The interactive HTML graph: one self-contained file built on the vendored
/// standard library (`decisions/the-html-graph-is-built-on-a-vendored-standard-library`):
/// cytoscape renders, and the ELK layered layout — reached through the
/// cytoscape-elk wrapper — places the nodes, with nothing hand-rolled beyond
/// configuration and a small init glue. Rust computes no geometry: it emits
/// the three pinned dist bundles verbatim (`assets/`, license headers
/// intact), a JSON data block, and the glue.
/// The data block carries one record per task — the name, the description,
/// the declared status, the role, the estimate, the done and crit flags,
/// the direct dependents in declaration order, and, when the schedule
/// artifact was handed in, its line (executor, start, end, active days);
/// absent fields are omitted — plus the canonicalized edges with their crit
/// marks, the flow direction (`DOWN`/`RIGHT`), and the legend lines. The
/// glue sizes each node's box to its two label lines (`role · estimate`
/// under the name) by measuring text the way cytoscape itself measures a
/// label, and leaves everything else to the libraries: ELK positions and
/// orders the layers along the direction, cytoscape draws and interacts —
/// pan, wheel zoom, fit on load, hover highlight (1-hop by default, the
/// transitive downstream on Shift via `successors()`), click-to-pin detail
/// card, status colours, done muting, class-picked arrowheads, corner
/// legend. The artifact opens offline; the accepted cost is ~2 MB of
/// embedded bundles.
///
/// The schedule artifact, when handed one, marks the critical path: nodes
/// named in the artifact's `critical-path` chain wear `crit`, and an edge
/// wears `crit` exactly when it is a consecutive pair of the chain — the
/// chain is emitted dependency-first and the scheduler steps only along
/// real dependencies, so consecutive pairs are real edges by construction;
/// no heuristic. `None` marks nothing and prints no artifact data. Done
/// tasks are never on the chain (the scheduler leaves them off), so done
/// muting and `crit` never collide.
///
/// The bytes stay deterministic: the model's declaration order (the
/// `BTreeMap`s), the canonicalized edge set, and the pinned bundles — same
/// plan, same blobs, same bytes.
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

    // The JSON data block: the structure the glue draws and the card reads.
    // Declaration order for the tasks (the `BTreeMap`'s key order), the
    // canonicalized edge set, an absent field printed no key at all.
    let records: Vec<JsonValue> = tasks
        .iter()
        .map(|(name, task)| {
            let mut rec = JsonMap::new();
            rec.insert("name".into(), json!(name));
            if let Some(desc) = &task.description {
                rec.insert("description".into(), json!(desc));
            }
            // The declared status, kebab-case like the artifact writes it;
            // todo declares nothing — it is the default look.
            if let Some(status) = task.status {
                if status != Status::Todo {
                    rec.insert("status".into(), json!(status_label(status)));
                }
            }
            rec.insert("role".into(), json!(task.role));
            if let Some(estimate) = task.estimate {
                rec.insert("estimate".into(), json!(estimate));
            }
            if task.status == Some(Status::Done) {
                rec.insert("done".into(), json!(true));
            }
            if crit_nodes.contains(name.as_str()) {
                rec.insert("crit".into(), json!(true));
            }
            // Direct dependents, declaration order — the raw adjacency.
            let downs: Vec<&str> = tasks
                .iter()
                .filter(|(_, t)| t.depends_on.iter().any(|d| d == name))
                .map(|(dependent, _)| dependent.as_str())
                .collect();
            if !downs.is_empty() {
                rec.insert("downs".into(), json!(downs));
            }
            // The artifact line, when handed one; absent fields omitted.
            if let Some(line) = schedule.and_then(|s| s.tasks.get(name.as_str())) {
                if let Some(executor) = &line.executor {
                    rec.insert("executor".into(), json!(executor));
                }
                if let Some(start) = line.start {
                    rec.insert("start".into(), json!(start.to_string()));
                }
                if let Some(end) = line.end {
                    rec.insert("end".into(), json!(end.to_string()));
                }
                if let Some(days) = line.active_days {
                    rec.insert("days".into(), json!(days));
                }
            }
            JsonValue::Object(rec)
        })
        .collect();
    let edge_records: Vec<JsonValue> = edges
        .iter()
        .map(|&(from, to)| {
            let mut rec = JsonMap::new();
            rec.insert("from".into(), json!(from));
            rec.insert("to".into(), json!(to));
            if crit_edges.contains(&(from, to)) {
                rec.insert("crit".into(), json!(true));
            }
            JsonValue::Object(rec)
        })
        .collect();

    // The corner legend, one line per entry so a future key joins as one
    // more push: the estimate unit, always; then a line per colour the page
    // actually shows — the muted done look, the in-progress blue, the
    // blocked orange, the red backbone — each only when something carries
    // it. The glue prints them as-is.
    let mut legend = vec![format!("unit: {}", plan.unit)];
    if tasks.values().any(|t| t.status == Some(Status::Done)) {
        legend.push("grey: done".to_string());
    }
    if tasks.values().any(|t| t.status == Some(Status::InProgress)) {
        legend.push("blue: in-progress".to_string());
    }
    if tasks.values().any(|t| t.status == Some(Status::Blocked)) {
        legend.push("orange: blocked".to_string());
    }
    if schedule.is_some_and(|s| !s.critical_path.is_empty()) {
        legend.push("red: critical path".to_string());
    }

    let data = json!({
        // The ELK layered direction: only the axis changes with `Direction`.
        "direction": match direction {
            Direction::Tb => "DOWN",
            Direction::Lr => "RIGHT",
        },
        "tasks": records,
        "edges": edge_records,
        "legend": legend,
    });
    let data_json = serde_json::to_string(&data).expect("the data block serializes");
    // The block rides in a <script type="application/json"> element: the
    // three characters that could close it or confuse the HTML parser are
    // written as \u escapes inside the string literals, which JSON.parse
    // reads back as the same characters.
    let data_json = data_json
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e");

    let mut out = String::from(HTML_HEAD);
    out.push_str("<script type=\"application/json\" id=\"graph-data\">");
    out.push_str(&data_json);
    out.push_str("</script>\n");
    out.push_str(HTML_TAIL);
    out
}

/// The head of the HTML graph page: the shell (background, legend, detail
/// card, the cytoscape container), the attribution note, and the three
/// vendored bundles embedded verbatim — cytoscape first (it defines the
/// global the wrapper registers against), then the ELK engine (it defines
/// the `ELK` global the wrapper's UMD factory reads), then the wrapper.
/// The bundles are pinned dist builds committed under `assets/`; nothing
/// here fetches from the network.
const HTML_HEAD: &str = concat!(
    r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>task dependency graph</title>
<style>
html, body { margin: 0; height: 100%; background: #fafaf7; }
body { font-family: ui-monospace, Menlo, Consolas, monospace; overflow: hidden; }
#cy { position: absolute; top: 0; left: 0; width: 100%; height: 100%; }
#legend { position: fixed; top: 12px; right: 16px; padding: 6px 10px; font-size: 11px; line-height: 1.7; color: #5b6470; background: rgba(255,255,255,0.9); border: 1px solid #c4cad2; border-radius: 6px; pointer-events: none; }
#card { position: fixed; left: 16px; bottom: 16px; max-width: 360px; padding: 6px 10px; font-size: 11px; line-height: 1.7; color: #5b6470; background: rgba(255,255,255,0.9); border: 1px solid #c4cad2; border-radius: 6px; pointer-events: none; display: none; }
#card.open { display: block; }
#card .title { color: #1c2430; }
#card .desc { white-space: pre-wrap; }
</style>
</head>
<body>
<div id="card"></div>
<div id="legend"></div>
<div id="cy"></div>
<!-- Rendered by planning-dsl. The graph draws with vendored cytoscape 3.34.4 (MIT),
elkjs 0.12.1 (EPL-2.0) and cytoscape-elk 2.3.0 (MIT), embedded verbatim below; the
bundles are pinned dist builds committed under assets/ with their license headers
intact. This file is self-contained and opens offline. -->
<script>
"#,
    include_str!("../assets/cytoscape-3.34.4.min.js"),
    "\n</script>\n<script>\n",
    include_str!("../assets/elk-0.12.1.bundled.js"),
    "\n</script>\n<script>\n",
    include_str!("../assets/cytoscape-elk-2.3.0.js"),
    "\n</script>\n"
);

/// The tail of the HTML graph page: the JSON data block rides in ahead of
/// this (Rust prints it between head and tail), then the init glue — the
/// only hand-written JS, and it only configures and reads. The glue builds
/// the cytoscape elements off the printed data, asks the ELK layered layout
/// for positions along the printed direction, and wires the interactions:
/// the hover glance (1-hop), the Shift transitive downstream
/// (`successors()`), the click-to-pin card, the corner legend, fit on load,
/// double-click and resize re-fit. Pan and wheel zoom are cytoscape's own.
const HTML_TAIL: &str = r#"<script>
(function () {
  var DATA = JSON.parse(document.getElementById('graph-data').textContent);

  // The card: one line per field present, built with
  // createElement/textContent — the description is plan prose, never
  // markup. Fixed bottom-left, part of the shell; hidden until opened.
  var card = document.getElementById('card');
  function cardLine(cls, text) {
    var div = document.createElement('div');
    if (cls) div.className = cls;
    div.textContent = text;
    card.appendChild(div);
  }
  function showCard(t) {
    while (card.firstChild) card.removeChild(card.firstChild);
    cardLine('title', t.name);
    if (t.status) cardLine(null, 'status: ' + t.status);
    if (t.description) cardLine('desc', t.description);
    if (t.executor) cardLine(null, 'executor: ' + t.executor);
    if (t.start && t.end) cardLine(null, t.start + ' \u2192 ' + t.end);
    if (t.days) cardLine(null, 'active days: ' + t.days);
    card.classList.add('open');
  }
  function hideCard() {
    card.classList.remove('open');
  }

  // The corner legend: the lines Rust decided on, one div per line.
  var legend = document.getElementById('legend');
  DATA.legend.forEach(function (line) {
    var div = document.createElement('div');
    div.textContent = line;
    legend.appendChild(div);
  });

  // Node sizing. Cytoscape's own label sizing (width/height 'label') is
  // deprecated in 3.34.4 and pads nothing — the text would sit on the
  // border — so the glue measures the two label lines the same way
  // cytoscape measures a label (a canvas 2d context, Math.ceil) and hands
  // each node its box. This is sizing, not layout: the positions, the
  // layering and the crossing order are ELK's.
  var FONT_FAMILY = 'ui-monospace, Menlo, Consolas, monospace';
  var FONT_SIZE = 11;
  var LINE_HEIGHT = 1.3; // the stylesheet's line-height, matched here
  var PAD_X = 10; // horizontal air inside the box, as the SVG renderer had
  var PAD_Y = 7;  // vertical air: two lines at line-height 1.3 breathe in a 40px box
  var MIN_W = 100;
  var measure = document.createElement('canvas').getContext('2d');
  measure.font = FONT_SIZE + 'px ' + FONT_FAMILY;
  // The second label line: `role · estimate`, the estimate written bare
  // ("6", "4.5"); without one, just the role. The unit is stated once, in
  // the corner legend.
  function subLabel(t) {
    return t.estimate == null ? t.role : t.role + ' \u00b7 ' + t.estimate;
  }
  function boxWidth(t) {
    // The box follows the longer of the two label lines.
    var lines = [t.name, subLabel(t)];
    var w = 0;
    for (var i = 0; i < lines.length; i++) {
      w = Math.max(w, Math.ceil(measure.measureText(lines[i]).width));
    }
    return Math.max(w + 2 * PAD_X, MIN_W);
  }
  var BOX_H = Math.ceil(FONT_SIZE * (1 + LINE_HEIGHT) + 2 * PAD_Y); // 40px

  // The elements: one node per task record, one edge per printed edge —
  // the classes additive, crit (the schedule-derived backbone) first, the
  // status class second; todo declares neither.
  var tasks = {};
  var elements = [];
  DATA.tasks.forEach(function (t) {
    tasks[t.name] = t;
    var classes = [];
    if (t.crit) classes.push('crit');
    if (t.status) classes.push(t.status);
    elements.push({
      data: {
        id: t.name,
        // The name over its `role · estimate` line: one label, wrapped at
        // the newline, in a box sized to the longer line.
        label: t.name + '\n' + subLabel(t),
        w: boxWidth(t),
        h: BOX_H
      },
      classes: classes.join(' ')
    });
  });
  DATA.edges.forEach(function (e, i) {
    elements.push({
      data: { id: 'e' + i, source: e.from, target: e.to },
      classes: e.crit ? 'crit' : ''
    });
  });

  var FIT_PAD = 48; // the fit margin the hand-rolled renderer used
  function fit() { cy.fit(FIT_PAD); }

  var cy = window.cytoscape({
    container: document.getElementById('cy'),
    elements: elements,
    // A read-only view: the positions are ELK's, nothing grabs a node.
    autoungrabify: true,
    layout: {
      name: 'elk',
      fit: false, // the glue fits, at stop, with its own padding
      padding: FIT_PAD,
      elk: {
        algorithm: 'layered',
        'elk.direction': DATA.direction,
        // Spacing, visibly airy — the hand-rolled renderer was widened
        // 1.5x at the maintainer's request (H_GAP 60 -> 90, ROW_H 56 ->
        // 84); ELK gets that air, and a little more layer to layer.
        'elk.spacing.nodeNode': 90,
        'elk.layered.spacing.nodeNodeBetweenLayers': 100
      },
      stop: function () { fit(); }
    },
    style: [
      {
        selector: 'node',
        style: {
          shape: 'round-rectangle',
          width: 'data(w)',
          height: 'data(h)',
          'background-color': '#ffffff',
          'border-width': 1.5,
          'border-color': '#5b6470',
          label: 'data(label)',
          'text-wrap': 'wrap',
          'text-valign': 'center',
          'text-halign': 'center',
          'line-height': LINE_HEIGHT,
          'font-family': FONT_FAMILY,
          'font-size': FONT_SIZE,
          color: '#1c2430'
        }
      },
      // The muted done look: the node keeps its place and its edges.
      {
        selector: 'node.done',
        style: {
          'background-color': '#f1f2ee',
          'border-color': '#c4cad2',
          color: '#9aa4b2'
        }
      },
      { selector: 'node.in-progress', style: { 'border-color': '#2563eb', color: '#2563eb' } },
      { selector: 'node.blocked', style: { 'border-color': '#ea580c', color: '#ea580c' } },
      // Crit is the backbone: declared after the status colours, it wins
      // over them; todo keeps the default look.
      { selector: 'node.crit', style: { 'border-color': '#b91c1c', color: '#b91c1c' } },
      {
        selector: 'edge',
        style: {
          width: 1.5,
          'line-color': '#9aa4b2',
          'target-arrow-shape': 'triangle',
          'target-arrow-color': '#9aa4b2',
          'curve-style': 'straight'
        }
      },
      { selector: 'edge.crit', style: { 'line-color': '#b91c1c', 'target-arrow-color': '#b91c1c' } },
      // The dim state: everything recedes; the crit backbone holds 0.4 so
      // it stays readable while a neighbourhood is inspected.
      { selector: '.dim', style: { opacity: 0.15 } },
      { selector: '.dim.crit', style: { opacity: 0.4 } },
      // The hot neighbourhood: the orange of the glance.
      {
        selector: 'node.hot',
        style: {
          'background-color': '#fff3e0',
          'border-color': '#b45309',
          color: '#7c3a00',
          opacity: 1
        }
      },
      {
        selector: 'edge.hot',
        style: {
          'line-color': '#b45309',
          'target-arrow-color': '#b45309',
          opacity: 1
        }
      }
    ]
  });

  // The glance and the closure. `hot` is the node whose highlight stands —
  // the one under the pointer, or the pin restored after leaving it.
  var shift = false, hot = null, pinned = null;
  function setHot(node) {
    // Hovering overrides whatever stood before it; leaving restores the
    // pin — the card rides the same override.
    cy.elements().removeClass('hot dim');
    hot = node;
    cy.elements().addClass('dim');
    // The cheap glance: the node and whatever touches it. Shift held: the
    // full transitive downstream — what moves if this slips. Upstream
    // stays out; the critical path tells that story.
    var eles = shift ? node.union(node.successors()) : node.closedNeighbourhood();
    eles.addClass('hot');
    showCard(tasks[node.id()]);
  }
  function clearHot() {
    hot = null;
    cy.elements().removeClass('hot dim');
    hideCard();
  }
  // The pin: a click leaves a node's highlight standing after mouseleave.
  function applyPin() {
    clearHot();
    if (pinned !== null) setHot(pinned);
  }
  // A shift press or release re-runs the standing highlight under the new
  // mode: it grows to the subtree while held and collapses to the 1-hop
  // glance on release.
  function setShift(on) {
    if (shift === on) return;
    shift = on;
    if (hot !== null) setHot(hot);
  }

  cy.on('mouseover', 'node', function (evt) { setHot(evt.target); });
  cy.on('mouseout', 'node', function () { applyPin(); });
  cy.on('tap', 'node', function (evt) {
    pinned = (pinned !== null && pinned.id() === evt.target.id()) ? null : evt.target;
    applyPin();
  });
  // A tap on the background clears the pin.
  cy.on('tap', function (evt) {
    if (evt.target === cy) {
      pinned = null;
      applyPin();
    }
  });

  window.addEventListener('keydown', function (ev) {
    if (ev.key === 'Shift') setShift(true);
  });
  window.addEventListener('keyup', function (ev) {
    if (ev.key === 'Shift') setShift(false);
  });

  document.getElementById('cy').addEventListener('dblclick', fit);
  window.addEventListener('resize', function () { cy.resize(); fit(); });
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
