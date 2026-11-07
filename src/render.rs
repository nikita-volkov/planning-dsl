//! Renderers: derived artifacts — a mermaid flowchart of the task dependency
//! DAG, a self-contained interactive HTML graph, a mermaid gantt chart of the
//! schedule, and a self-contained interactive HTML gantt. All are pure
//! functions: same input in, byte-identical file out. The flowchart is a
//! function of the plan; the gantt is a function of the schedule artifact
//! alone; the HTML graph is a function of the plan plus the artifact when
//! handed one — its critical path and the per-task lines the detail card
//! reads (`specs/scheduler.md`: renderers consume it, nothing recomputes
//! it); the HTML gantt draws with the vendored dhtmlx-gantt 10.0.3
//! (`decisions/the-html-gantt-is-a-vendored-dhtmlx-gantt.md`) — the
//! artifact alone fixes every date, the plan is read only for the
//! dependency arrows — and none re-derives a date, a capacity or an
//! assignment. The determinism is carried by the model's declaration order
//! (the `DeclarationMap`) and by canonicalizing the edge set, never by
//! hashing or sorting at the end.
//!
//! Renderers assume a valid input (the CLI gates on `validate`, and the
//! gantt's CLI path schedules first); defensively, a `depends-on` naming no
//! task is skipped, never rendered as a ghost node.

use std::collections::{BTreeMap, BTreeSet};

use chrono::Duration;
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

/// The interactive HTML gantt: one self-contained file built on the vendored
/// dhtmlx-gantt 10.0.3 (MIT, Community edition —
/// `decisions/the-html-gantt-is-a-vendored-dhtmlx-gantt.md`). The
/// configuration is the comparison file the maintainer chose on 2026-11-04
/// (`research/gantt-library-comparison`, commit 9a6157f) fed from this
/// repo's data — the same look and behavior, not a re-design. The community
/// bundle draws everything; Rust emits the pinned dist bundle verbatim
/// (`assets/dhtmlx-gantt-10.0.3/`, the license file beside it), a JSON data
/// block, and the init glue.
///
/// The data block: one `project` summary row per executor — executor
/// grouping is emulated, the community edition bundles no `groupBy` — in
/// first-appearance order along the artifact's task declaration order, the
/// mermaid gantt's precedent; one record per dated task under its
/// executor's row, declaration order. Done tasks carry no dates in the
/// artifact and are off the chart entirely. `end_date` is written as the
/// artifact's end + 1 day — dhtmlx's end is exclusive, the artifact's is
/// inclusive — and the tooltip and the quick_info card report the
/// artifact's inclusive end from the meta, which carries every field they
/// show: executor, start, end, active-days, status, and the critical flag.
/// The links are the canonicalized edges whose both endpoints have a bar —
/// a done task has no dates, so no bar, so no link. A dated task the
/// artifact leaves unassigned (the scheduler never produces one) renders
/// defensively top-level, in no group.
///
/// The glue configures, reads, and decides nothing: `readonly: true` (the
/// artifact is a computed, regenerable view), `auto_scheduling: false` (the
/// artifact's dates are the computed truth, never recomputed client-side),
/// the zoom extension at three two-tier levels wired to day/week/month
/// buttons (the community bundle adds no zoom UI of its own), the status
/// colors and the critical-path bars through the `task_class` template, and
/// the today tint through `scale_cell_class`/`timeline_cell_class` — drawn
/// by the viewer's browser from the viewer's clock, only inside the window,
/// so the file's bytes stay deterministic.
///
/// The vendored stylesheet `@font-face`s Inter from fonts.gstatic.com; those
/// rules are stripped at render time (`strip_remote_font_faces`) and the
/// system-font fallback the stylesheet's own stack names takes over — the
/// emitted file makes zero network references and opens offline.
///
/// The bytes stay deterministic: the model's declaration order, the
/// canonicalized edge set, the pinned bundle and the stripped stylesheet —
/// same plan, same artifact, same bytes.
pub fn gantt_html(plan: &Plan, schedule: &Schedule) -> String {
    // The window: the artifact's span; with nothing left to do, its single
    // first day.
    let window_start = schedule.start;
    let window_end = schedule.plan_end.unwrap_or(schedule.start) + Duration::days(1);

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
    // one) renders, defensively, top-level in no group.
    let mut executors: Vec<Option<&str>> = Vec::new();
    for &(_, line) in &dated {
        let executor = line.executor.as_deref();
        if !executors.contains(&executor) {
            executors.push(executor);
        }
    }
    executors.sort_by_key(|g| g.is_some());

    // The crit chain, read off the artifact: records named in it wear the
    // critical flag, painted by the task_class template.
    let crit_nodes: BTreeSet<&str> = schedule.critical_path.iter().map(String::as_str).collect();

    // The links: the canonicalized edges whose both endpoints have a bar —
    // a done task has no dates, so no bar, so no link. Ids count from 1.
    let dated_set: BTreeSet<&str> = dated.iter().map(|&(name, _)| name).collect();
    let links: Vec<JsonValue> = edges(plan)
        .iter()
        .filter(|&(from, to)| dated_set.contains(from) && dated_set.contains(to))
        .enumerate()
        .map(|(i, &(from, to))| {
            json!({"id": i + 1, "source": from, "target": to, "type": "0"})
        })
        .collect();

    // The data records: a project row per executor, then every dated task
    // under its executor's row — declaration order, the tree the grid draws.
    let mut data: Vec<JsonValue> = Vec::new();
    for name in executors.iter().flatten() {
        data.push(json!({
            "id": format!("grp-{name}"),
            "text": name,
            "type": "project",
            "render": "project",
            "open": true,
            "meta": {"header": true},
        }));
    }
    for &(task, line) in &dated {
        let start = line.start.expect("dated");
        let end = line.end.expect("dated");
        let mut meta = JsonMap::new();
        if let Some(executor) = &line.executor {
            meta.insert("executor".into(), json!(executor));
        }
        meta.insert("start".into(), json!(start.to_string()));
        meta.insert("end".into(), json!(end.to_string()));
        if let Some(days) = line.active_days {
            meta.insert("active_days".into(), json!(days));
        }
        meta.insert("status".into(), json!(status_label(line.status)));
        meta.insert("critical".into(), json!(crit_nodes.contains(task)));
        let mut rec = JsonMap::new();
        rec.insert("id".into(), json!(task));
        rec.insert("text".into(), json!(task));
        if let Some(executor) = &line.executor {
            rec.insert("parent".into(), json!(format!("grp-{executor}")));
        }
        rec.insert("start_date".into(), json!(format!("{start} 00:00:00")));
        // dhtmlx's end_date is exclusive; the artifact's end is inclusive —
        // the bar must cover start..end, the tooltip reports the artifact's
        // end from the meta.
        rec.insert(
            "end_date".into(),
            json!(format!("{} 00:00:00", end + Duration::days(1))),
        );
        rec.insert("meta".into(), JsonValue::Object(meta));
        data.push(JsonValue::Object(rec));
    }

    let data_json =
        serde_json::to_string(&json!({"data": data, "links": links})).expect("the data block serializes");
    // The block rides in a <script> element: a literal `</` in it could
    // close or confuse the parser — written as `<\/`, which JS reads back
    // as the same string.
    let data_json = data_json.replace("</", "<\\/");

    let mut out = String::from(GANTT_HEAD);
    out.push_str(&strip_remote_font_faces(DHTMLX_CSS));
    out.push_str(GANTT_SHELL);
    out.push_str(DHTMLX_JS);
    out.push_str(GANTT_GLUE_HEAD);
    out.push_str(&format!(
        "gantt.config.start_date = '{window_start} 00:00:00';\n\
         gantt.config.end_date = '{window_end} 00:00:00';\n",
    ));
    out.push_str(GANTT_GLUE_BODY);
    out.push_str(&data_json);
    out.push_str(GANTT_GLUE_TAIL);
    out
}

/// The vendored dhtmlx stylesheet with its remote `@font-face` rules
/// stripped: the bundle ships Inter from fonts.gstatic.com, and the emitted
/// file must make zero network references — stripped, the browser falls
/// back to the system fonts the stylesheet's own stack names (the icon font
/// ships as a data URI and stays). Deterministic by construction: a
/// balanced-brace scan that drops exactly the rules referencing the remote
/// host, byte-identical for the same input.
fn strip_remote_font_faces(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(at) = rest.find("@font-face") {
        out.push_str(&rest[..at]);
        let open = at + rest[at..].find('{').expect("a font-face rule opens");
        let mut depth = 0;
        let mut close = None;
        for (i, c) in rest[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(open + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        let close = close.expect("a font-face rule closes");
        if !rest[at..close].contains("fonts.gstatic.com") {
            out.push_str(&rest[at..close]);
        }
        rest = &rest[close..];
    }
    out.push_str(rest);
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
/// cytoscape renders, and the ELK layered layout — run by the glue directly —
/// places the nodes and routes the edges
/// (`decisions/the-html-graph-routes-edges-along-elks-bend-points.md`), with
/// nothing hand-rolled beyond configuration and a small init glue. Rust
/// computes no geometry: it emits the pinned dist bundles verbatim (`assets/`,
/// license headers intact), a JSON data block, and the glue.
/// The data block carries one record per task — the name, the description,
/// the declared status, the role, the estimate, the done and crit flags,
/// the direct dependents in declaration order, and, when the schedule
/// artifact was handed in, its line (executor, start, end, active days);
/// absent fields are omitted — plus the canonicalized edges with their crit
/// marks, the flow direction (`DOWN`/`RIGHT`), and the legend lines. The
/// glue sizes each node's box to its two label lines (`role · estimate`
/// under the name) by measuring text the way cytoscape itself measures a
/// label, and leaves everything else to the libraries: ELK positions and
/// orders the layers along the direction and routes every edge, cytoscape
/// draws and interacts —
/// pan, wheel zoom, fit on load, hover highlight (1-hop by default, the
/// transitive downstream on Shift via `successors()`), click-to-pin detail
/// card, node dragging — the edges re-interpolate off the same data and the
/// arrangement is the viewer's local adjustment, never persisted —
/// status colours, done muting, class-picked arrowheads, corner
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
/// card, the cytoscape container), the attribution note, and the two
/// vendored bundles embedded verbatim — cytoscape (the viewer) first, then
/// the ELK engine (the `ELK` global the init glue lays out with). The
/// bundles are pinned dist builds committed under `assets/`; nothing here
/// fetches from the network.
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
<!-- Rendered by planning-dsl. The graph draws with vendored cytoscape 3.34.4 (MIT)
and elkjs 0.12.1 (EPL-2.0), embedded verbatim below; the bundles are pinned dist
builds committed under assets/ with their license headers intact. This file is
self-contained and opens offline. -->
<script>
"#,
    include_str!("../assets/cytoscape-3.34.4.min.js"),
    "\n</script>\n<script>\n",
    include_str!("../assets/elk-0.12.1.bundled.js"),
    "\n</script>\n"
);

/// The tail of the HTML graph page: the JSON data block rides in ahead of
/// this (Rust prints it between head and tail), then the init glue — the
/// only hand-written JS, and it only configures and reads. The glue builds
/// the cytoscape elements off the printed data, runs the vendored ELK
/// layered layout along the printed direction, applies its node positions
/// and maps its edge sections onto cytoscape's `segments` curve style, and
/// wires the interactions: the hover glance (1-hop), the Shift transitive
/// downstream (`successors()`), the click-to-pin card, node dragging (the
/// edges follow; the arrangement is the viewer's local adjustment), the
/// corner legend, fit on load, double-click and resize re-fit. Pan and wheel
/// zoom are cytoscape's own.
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
    // The nodes are grabbable: ELK's arrangement is the starting point, and
    // dragging a box is the viewer's local adjustment. The edges ride the
    // same weights and distances — with `edge-distances: node-position` the
    // segment points interpolate between the node centers, so a dragged box
    // drags its edges along. Nothing is persisted: a re-render lays ELK's
    // arrangement out again.
    // The nodes carry no positions yet — ELK delivers them below; preset
    // keeps cytoscape from scattering them in the meantime, and the glue
    // fits once the layout has landed.
    layout: { name: 'preset', fit: false },
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
          // The edges ride ELK's computed routes: `segments` draws the
          // polyline through the segment points the glue maps from ELK's
          // sections below. `node-position` makes those points interpolate
          // between the node centers (the space the mapping inverts), and
          // the manual source/target endpoints pin the drawn ends to ELK's
          // border points instead of a center-ray projection — the
          // projection misses whenever ELK spreads ports off-center.
          'curve-style': 'segments',
          'edge-distances': 'node-position',
          'segment-weights': 'data(w)',
          'segment-distances': 'data(d)',
          'source-endpoint': 'data(send)',
          'target-endpoint': 'data(tend)'
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
  var cyNodes = cy.nodes(), cyEdges = cy.edges();

  // The layout: the vendored ELK layered engine, run directly. The
  // cytoscape-elk wrapper that used to reach it applies node positions only
  // — its adapter drops the computed edge routes — so the glue feeds ELK
  // its own JSON graph (the same measured boxes, edges as id/source/target)
  // and maps the routes back onto cytoscape itself
  // (decisions/the-html-graph-routes-edges-along-elks-bend-points.md). The
  // graph is flat — every node a child of the root — so ELK's node boxes
  // and edge sections all come back in root coordinates.
  new ELK().layout({
    id: 'root',
    layoutOptions: {
      algorithm: 'layered',
      'elk.direction': DATA.direction,
      // Spacing, visibly airy — the hand-rolled renderer was widened
      // 1.5x at the maintainer's request (H_GAP 60 -> 90, ROW_H 56 ->
      // 84); ELK gets that air, and a little more layer to layer.
      'elk.spacing.nodeNode': 90,
      'elk.layered.spacing.nodeNodeBetweenLayers': 100
    },
    children: DATA.tasks.map(function (t) {
      return { id: t.name, width: boxWidth(t), height: BOX_H };
    }),
    edges: DATA.edges.map(function (e, i) {
      return { id: 'e' + i, sources: [e.from], targets: [e.to] };
    })
  }).then(function (graph) {
    // ELK positions a node by its top-left corner; cytoscape wants the
    // center. ELK routes each edge in sections — start point, bends, end
    // point. Cytoscape's `segments` curve style has no absolute-points
    // property: it computes its points from segment weights and distances,
    //   segpt = src + w * (tgt - src) + dist * rot90(unit(tgt - src)),
    // and `route` below is that formula's exact inverse, so each mapped
    // pair reconstructs its ELK point bit-for-bit.
    function route(src, tgt, pts) {
      var dx = tgt.x - src.x, dy = tgt.y - src.y;
      var l2 = dx * dx + dy * dy;
      if (l2 === 0) return null;
      var ux = dx / Math.sqrt(l2), uy = dy / Math.sqrt(l2);
      var ws = [], ds = [];
      for (var i = 0; i < pts.length; i++) {
        var px = pts[i].x - src.x, py = pts[i].y - src.y;
        ws.push((px * dx + py * dy) / l2);
        ds.push(py * ux - px * uy);
      }
      return { w: ws, d: ds };
    }
    var nodesById = {};
    graph.children.forEach(function (n) { nodesById[n.id] = n; });
    cy.batch(function () {
      graph.children.forEach(function (n, i) {
        cyNodes[i].position({ x: n.x + n.width / 2, y: n.y + n.height / 2 });
      });
      graph.edges.forEach(function (e, i) {
        var s = e.sections && e.sections[0];
        if (!s) return;
        var src = nodesById[e.sources[0]], tgt = nodesById[e.targets[0]];
        var S = { x: src.x + src.width / 2, y: src.y + src.height / 2 };
        var T = { x: tgt.x + tgt.width / 2, y: tgt.y + tgt.height / 2 };
        var bends = s.bendPoints || [];
        // A straight section carries no bends; its midpoint keeps the
        // segments machinery fed — an empty point list computes NaN ends
        // and cytoscape refuses to draw the edge at all.
        var pts = bends.length ? bends : [
          { x: (s.startPoint.x + s.endPoint.x) / 2, y: (s.startPoint.y + s.endPoint.y) / 2 }
        ];
        var mapped = route(S, T, pts);
        if (!mapped) return;
        cyEdges[i].data({
          w: mapped.w,
          d: mapped.d,
          // The border anchorings as offsets from the node centers: with
          // these the drawn line starts and ends exactly on ELK's points,
          // and the arrowhead lands on the border pointing along ELK's
          // final approach.
          send: (s.startPoint.x - S.x) + 'px ' + (s.startPoint.y - S.y) + 'px',
          tend: (s.endPoint.x - T.x) + 'px ' + (s.endPoint.y - T.y) + 'px'
        });
      });
    });
    fit(); // the layout has landed; also forces the style's re-evaluation
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

/// The vendored dhtmlx-gantt 10.0.3 dist build, verbatim: the pinned files
/// committed under `assets/dhtmlx-gantt-10.0.3/` with the license file
/// (sha256s recorded in `research/gantt-library-comparison/README.md`).
/// Nothing here fetches from the network; the stylesheet's remote
/// `@font-face` rules are stripped at render time (`strip_remote_font_faces`).
const DHTMLX_JS: &str = include_str!("../assets/dhtmlx-gantt-10.0.3/dhtmlxgantt.js");
const DHTMLX_CSS: &str = include_str!("../assets/dhtmlx-gantt-10.0.3/dhtmlxgantt.css");

/// The head of the HTML gantt page, up to the vendored stylesheet: the
/// comparison file's shell, the library named in the title. Separate from
/// the glue so the two halves cannot drift.
const GANTT_HEAD: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>schedule gantt — dhtmlx-gantt 10.0.3</title>
<style>
"#;

/// After the (stripped) vendored stylesheet: the gantt's own styling — the
/// comparison's custom css verbatim — then the shell, the attribution note,
/// and the script tag the vendored bundle rides in behind.
const GANTT_SHELL: &str = r#"
</style>
<style>
html, body { margin: 0; padding: 0; height: 100%; background: #fafaf7; font-family: ui-monospace, Menlo, Consolas, monospace; }
#gantt-here { width: 100%; height: 100vh; }
/* status colors — the native gantt's palette, via the task_class hook */
.gantt_task_line.st-todo { background-color: #5b6470; }
.gantt_task_line.st-in-progress { background-color: #2563eb; }
.gantt_task_line.st-blocked { background-color: #ea580c; }
.gantt_task_line.crit { box-shadow: inset 0 0 0 2px #b91c1c; }
/* executor group rows: emulated with project summary rows (the community
   edition bundles no groupBy plugin) */
.gantt_task_line.grp-row { background-color: #eceee6; }
.gantt_task_line .gantt_task_content { color: #ffffff; font-size: 11px; }
.gantt_task_line.grp-row .gantt_task_content { color: #5b6470; font-weight: 700; }
.dhx-today-cell { background: #f3d1d1; }
#zoom-buttons { position: fixed; top: 8px; right: 12px; z-index: 5; }
#zoom-buttons button { font: 11px ui-monospace, Menlo, Consolas, monospace; margin-left: 4px; padding: 2px 8px; border: 1px solid #c4cad2; border-radius: 4px; background: #fff; color: #5b6470; cursor: pointer; }
</style>
</head>
<body>
<div id="gantt-here"></div>
<!-- Rendered by planning-dsl. The gantt draws with vendored
dhtmlx-gantt 10.0.3 (MIT, Community edition), embedded verbatim below; the bundle
is a pinned dist build committed under assets/dhtmlx-gantt-10.0.3/ with its
license file. The stylesheet's remote @font-face rules are stripped at render
time, so offline the file falls back to a system font. This file is
self-contained and opens offline. -->
<script>
"#;

/// After the vendored bundle: the init glue — the only hand-written JS, and
/// it only configures and reads. The window dates and the data block are
/// printed between the glue's halves (Rust pushes them ahead of
/// `GANTT_GLUE_BODY` and `GANTT_GLUE_TAIL`).
const GANTT_GLUE_HEAD: &str = r#"
</script>
<script>
gantt.plugins({ tooltip: true, quick_info: true });
gantt.config.date_format = '%Y-%m-%d %H:%i:%s';
"#;

const GANTT_GLUE_BODY: &str = r#"gantt.config.readonly = true;        // the artifact is a computed, regenerable view
gantt.config.auto_scheduling = false; // the dates come from the artifact, not recomputed
// zoom levels through the bundled zoom extension (v10 community): two tiers
// per level, month over day like the native gantt at level 0
if (gantt.ext && gantt.ext.zoom) {
  gantt.ext.zoom.init({
    levels: [
      [
        { unit: 'month', step: 1, format: '%M %Y' },
        { unit: 'day', step: 1, format: '%j' }
      ],
      [
        { unit: 'month', step: 1, format: '%M %Y' },
        { unit: 'week', step: 1, format: function (d) { return gantt.date.date_to_str('%d %M')(d); } }
      ],
      [
        { unit: 'year', step: 1, format: '%Y' },
        { unit: 'month', step: 1, format: '%M' }
      ]
    ]
  });
  gantt.ext.zoom.setLevel(0);
} else {
  gantt.config.scales = [
    { unit: 'month', step: 1, format: '%M %Y' },
    { unit: 'day', step: 1, format: '%j' }
  ];
}
gantt.config.columns = [
  { name: 'text', label: 'Task', tree: true, width: 220 },
  { name: 'start_date', label: 'Start', width: 90, align: 'center' },
  { name: 'end_date', label: 'End', width: 90, align: 'center' }
];
gantt.templates.task_class = function (start, end, task) {
  if (!task.meta || task.meta.header) return 'grp-row';
  return (task.meta.critical ? 'crit ' : '') + 'st-' + task.meta.status;
};
// The artifact's fields, one line each — the hover tooltip and the click
// quick_info card show the same list.
function taskDetails(task) {
  var m = task.meta;
  return '<b>' + task.text + '</b>' +
    '<br/>executor: ' + m.executor +
    '<br/>start: ' + m.start +
    '<br/>end: ' + m.end +
    '<br/>active-days: ' + m.active_days +
    '<br/>status: ' + m.status +
    (m.critical ? '<br/>critical path: yes' : '');
}
gantt.templates.tooltip_text = function (start, end, task) {
  if (!task.meta || task.meta.header) return task.text;
  return taskDetails(task);
};
// The quick_info card shows the same fields as the tooltip; the bundle's
// default card carries only the name and the time range.
gantt.templates.quick_info_title = function (start, end, task) {
  return task.text;
};
gantt.templates.quick_info_date = function () { return ''; };
gantt.templates.quick_info_content = function (start, end, task) {
  if (!task.meta || task.meta.header) return task.text;
  return taskDetails(task);
};

const DATA = "#;

const GANTT_GLUE_TAIL: &str = r#";

// today marker, drawn by the viewer's browser from the viewer's clock; only
// when it falls inside the window — the native gantt's rule. The community
// bundle ships neither the marker extension nor addTaskLayer, so the today
// date is highlighted through the scale/timeline cell-class templates
// instead of a full-height line.
var TODAY_CELL_CLASS = 'dhx-today-cell';
function isToday(date) {
  return gantt.date.day_start(new Date(date)).valueOf() === gantt.date.day_start(new Date()).valueOf();
}
gantt.templates.scale_cell_class = function (date) {
  if (isToday(date)) return TODAY_CELL_CLASS;
};
gantt.templates.timeline_cell_class = function (item, date) {
  if (isToday(date)) return TODAY_CELL_CLASS;
};

gantt.init('gantt-here');
gantt.parse(DATA);

// view-level buttons (the community bundle adds no zoom UI of its own)
var zoomBtns = document.createElement('div');
zoomBtns.id = 'zoom-buttons';
['day', 'week', 'month'].forEach(function (name, i) {
  var b = document.createElement('button');
  b.textContent = name;
  b.onclick = function () { gantt.ext.zoom.setLevel(i); };
  zoomBtns.appendChild(b);
});
document.body.appendChild(zoomBtns);
</script>
</body>
</html>
"#;
