#!/usr/bin/env python3
"""Generate the frappe-gantt and dhtmlx-gantt versions of the house-renovation
demo gantt, for the library comparison in this folder.

Reads the demo's schedule artifact and plan (a hand-rolled reader for these
two known files — indented keys, no yaml library), inlines the vendored
library JS + CSS verbatim, embeds the task data as JSON, and writes:

    house-renovation.gantt.frappe.html
    house-renovation.gantt.dhtmlx.html

Stdlib only, no timestamps in the output — the bytes are a pure function of
the inputs (the vendored files and the two demo yamls).

Usage:  python3 generate.py
"""

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent
DEMO = ROOT.parent / "house-renovation-demo"
VENDOR = ROOT / "vendor"

FRAPPE_PKG = VENDOR / "frappe-gantt-1.2.2"
DHTMLX_PKG = VENDOR / "dhtmlx-gantt-10.0.3"

FRAPPE_OUT = ROOT / "house-renovation.gantt.frappe.html"
DHTMLX_OUT = ROOT / "house-renovation.gantt.dhtmlx.html"


# ---------------------------------------------------------------------------
# tiny readers for the two known demo files
# ---------------------------------------------------------------------------

def strip_inline_comment(value):
    # inline comments in these files always start with ' #'
    if " #" in value:
        value = value.split(" #")[0]
    return value.strip()


def read_indented(path):
    """Yield (indent, text) lines of a file, skipping blanks and comments."""
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        stripped = line.strip()
        yield len(line) - len(line.lstrip(" ")), stripped


def parse_schedule(path):
    """The schedule artifact: flat fixed shape.

    top scalars (start/deadline/plan-end/slippage-days/latest-start),
    critical-path: (- item)*, tasks: name -> {executor,start,end,active-days,status}.
    Returns scalars dict, critical path list, and an ordered task dict.
    """
    scalars = {}
    critical = []
    tasks = {}
    section = None
    current = None
    for indent, text in read_indented(path):
        if text.startswith("- "):  # list item (the critical-path list sits at indent 0)
            if section != "critical-path":
                sys.exit(f"schedule: unexpected list item {text!r}")
            critical.append(strip_inline_comment(text[2:]))
        elif indent == 0:
            if not text.endswith(":"):
                key, sep, value = text.partition(":")
                if not sep:
                    sys.exit(f"schedule: unhandled top-level line {text!r}")
                scalars[key] = strip_inline_comment(value)
                section = None
            else:
                section = text[:-1]
                current = None
        elif indent == 2 and section == "tasks":
            assert text.endswith(":"), f"unexpected task line: {text!r}"
            current = text[:-1]
            tasks[current] = {}
        elif indent == 4 and section == "tasks" and current is not None:
            key, _, value = text.partition(":")
            tasks[current][key] = strip_inline_comment(value)
        else:
            sys.exit(f"schedule: unhandled line {text!r} (indent {indent})")
    return scalars, critical, tasks


def parse_plan_dependencies(path):
    """The plan yaml: only the tasks' depends-on edges (inline lists)."""
    edges = {}
    section = None
    current = None
    for indent, text in read_indented(path):
        if indent == 0:
            section = text[:-1] if text.endswith(":") else None
            current = None
        elif indent == 2 and section == "tasks":
            current = text[:-1] if text.endswith(":") else current
        elif indent == 4 and current is not None and text.startswith("depends-on:"):
            value = strip_inline_comment(text.partition(":")[2])
            assert value.startswith("[") and value.endswith("]"), f"not an inline list: {value!r}"
            names = [item.strip() for item in value[1:-1].split(",") if item.strip()]
            edges[current] = names
    return edges


# ---------------------------------------------------------------------------
# schedule -> model
# ---------------------------------------------------------------------------

def build_model():
    scalars, critical, tasks = parse_schedule(DEMO / "house-renovation-schedule.yaml")
    depends = parse_plan_dependencies(DEMO / "house-renovation.yaml")

    dated = [
        {"id": name, **fields}
        for name, fields in tasks.items()
        if fields.get("status") != "done"
    ]
    for task in dated:
        task["critical"] = task["id"] in critical

    # rows group by executor in first-appearance order along the artifact's
    # task declaration order — the native gantt's rule
    executors = []
    for task in dated:
        if task["executor"] not in executors:
            executors.append(task["executor"])

    # dependency arrows only where both endpoints have a bar (done tasks have
    # no dates, so no bar, so no arrow — the native gantt's behavior too)
    edges = [
        (src, dst)
        for dst, sources in depends.items()
        for src in sources
        if src in tasks and dst in tasks
        and tasks[src]["status"] != "done" and tasks[dst]["status"] != "done"
    ]
    return scalars, dated, executors, edges, critical


def check_model(scalars, dated, executors, edges, critical):
    assert scalars["start"] == "2026-11-05", scalars
    assert scalars["plan-end"] == "2027-02-03", scalars
    assert len(dated) == 23, len(dated)
    assert len(executors) == 7, executors
    assert len(critical) == 6, critical
    assert sum(1 for task in dated if task["critical"]) == 6
    for task in dated:
        for field in ("executor", "start", "end", "active-days", "status"):
            assert field in task, (task["id"], field)


def next_day(date_str):
    m = re.match(r"(\d{4})-(\d{2})-(\d{2})$", date_str)
    assert m, date_str
    import datetime
    d = datetime.date(int(m[1]), int(m[2]), int(m[3])) + datetime.timedelta(days=1)
    return d.isoformat()


def embed_json(data):
    # keep the JSON safe inside a <script> block
    return json.dumps(data, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")


def inline_vendor(text):
    # the vendor files carry no '</script' / '</style' sequences (checked);
    # embed verbatim
    return text


# ---------------------------------------------------------------------------
# frappe html
# ---------------------------------------------------------------------------

FRAPPE_TASKS_JS = """\
const TASKS = %(tasks)s;

new Gantt('#gantt', TASKS, {
  readonly: true,             // the artifact is a computed, regenerable view
  view_mode: 'Day',
  view_mode_select: true,     // expose the zoom levels in the UI
  today_button: true,
  date_format: 'YYYY-MM-DD',
  popup: function (ctx) {
    var m = ctx.task.meta;
    if (!m || m.header) return false;
    ctx.set_title(ctx.task.name);
    ctx.set_subtitle('');
    ctx.set_details(
      'executor: ' + m.executor +
      '<br/>start: ' + m.start +
      '<br/>end: ' + m.end +
      '<br/>active-days: ' + m.active_days +
      '<br/>status: ' + m.status +
      (m.critical ? '<br/>critical path: yes' : '')
    );
  },
});
"""

FRAPPE_CUSTOM_CSS = """\
html, body { margin: 0; padding: 12px; background: #fafaf7; font-family: ui-monospace, Menlo, Consolas, monospace; }
#gantt { background: #fff; border-radius: 8px; }
/* critical path — the native gantt's red stroke, via frappe's custom_class
   hook (single token: a multi-token custom_class throws in classList.add,
   which is why status colors go through frappe's per-task color field) */
.bar-wrapper.crit .bar { stroke: #b91c1c; stroke-width: 2; }
/* executor group headers: emulated with full-span pseudo tasks (frappe 1.2.2
   has no grouping option) */
.bar-wrapper.grp-header .bar { fill: #eceee6; outline: 1px solid #e7e7df; }
.bar-wrapper.grp-header .bar-label { fill: #5b6470; font-weight: 700; }
"""

STATUS_COLORS = {
    "todo": "#5b6470",
    "in-progress": "#2563eb",
    "blocked": "#ea580c",
    "done": "#c4cad2",
}


def frappe_html(scalars, dated, executors, edges):
    window_start = min(task["start"] for task in dated)
    window_end = max(task["end"] for task in dated)

    frappe_tasks = []
    for executor in executors:
        frappe_tasks.append({
            "id": f"grp-{executor}",
            "name": executor,
            "start": window_start,
            "end": window_end,
            "progress": 0,
            "custom_class": "grp-header",
            "meta": {"header": True},
        })
        for task in dated:
            if task["executor"] != executor:
                continue
            deps = [src for src, dst in edges if dst == task["id"]]
            frappe_tasks.append({
                "id": task["id"],
                "name": task["id"],
                "start": task["start"],
                "end": task["end"],
                "progress": 0,
                "color": STATUS_COLORS[task["status"]],
                "custom_class": "crit" if task["critical"] else "",
                "dependencies": deps,
                "meta": {
                    "executor": task["executor"],
                    "start": task["start"],
                    "end": task["end"],
                    "active_days": task["active-days"],
                    "status": task["status"],
                    "critical": task["critical"],
                },
            })

    js = FRAPPE_TASKS_JS % {"tasks": embed_json(frappe_tasks)}
    css = inline_vendor((FRAPPE_PKG / "frappe-gantt.css").read_text(encoding="utf-8"))
    lib = inline_vendor((FRAPPE_PKG / "frappe-gantt.umd.js").read_text(encoding="utf-8"))

    return f"""<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>schedule gantt — frappe-gantt 1.2.2</title>
<style>
{css}
</style>
<style>
{FRAPPE_CUSTOM_CSS}
</style>
</head>
<body>
<div id="gantt"></div>
<script>
{lib}
</script>
<script>
{js}
</script>
</body>
</html>
"""


# ---------------------------------------------------------------------------
# dhtmlx html
# ---------------------------------------------------------------------------

DHTMLX_INIT_JS = """\
gantt.plugins({ tooltip: true, quick_info: true });
gantt.config.date_format = '%%Y-%%m-%%d %%H:%%i:%%s';
gantt.config.start_date = '%(start)s 00:00:00';
gantt.config.end_date = '%(end)s 00:00:00';
gantt.config.readonly = true;        // the artifact is a computed, regenerable view
gantt.config.auto_scheduling = false; // the dates come from the artifact, not recomputed
// zoom levels through the bundled zoom extension (v10 community): two tiers
// per level, month over day like the native gantt at level 0
if (gantt.ext && gantt.ext.zoom) {
  gantt.ext.zoom.init({
    levels: [
      [
        { unit: 'month', step: 1, format: '%%M %%Y' },
        { unit: 'day', step: 1, format: '%%j' }
      ],
      [
        { unit: 'month', step: 1, format: '%%M %%Y' },
        { unit: 'week', step: 1, format: function (d) { return gantt.date.date_to_str('%%d %%M')(d); } }
      ],
      [
        { unit: 'year', step: 1, format: '%%Y' },
        { unit: 'month', step: 1, format: '%%M' }
      ]
    ]
  });
  gantt.ext.zoom.setLevel(0);
} else {
  gantt.config.scales = [
    { unit: 'month', step: 1, format: '%%M %%Y' },
    { unit: 'day', step: 1, format: '%%j' }
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
gantt.templates.tooltip_text = function (start, end, task) {
  if (!task.meta || task.meta.header) return task.text;
  var m = task.meta;
  return '<b>' + task.text + '</b>' +
    '<br/>executor: ' + m.executor +
    '<br/>start: ' + m.start +
    '<br/>end: ' + m.end +
    '<br/>active-days: ' + m.active_days +
    '<br/>status: ' + m.status +
    (m.critical ? '<br/>critical path: yes' : '');
};

const DATA = %(data)s;

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
"""

DHTMLX_CUSTOM_CSS = """\
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
"""


def dhtmlx_html(scalars, dated, executors, edges):
    window_start = scalars["start"]
    window_end = next_day(scalars["plan-end"])

    data = []
    for executor in executors:
        data.append({
            "id": f"grp-{executor}",
            "text": executor,
            "type": "project",
            "render": "project",
            "open": True,
            "meta": {"header": True},
        })
    for task in dated:
        data.append({
            "id": task["id"],
            "text": task["id"],
            "parent": f"grp-{task['executor']}",
            "start_date": f"{task['start']} 00:00:00",
            # dhtmlx end_date is exclusive; the artifact's end is inclusive
            "end_date": f"{next_day(task['end'])} 00:00:00",
            "meta": {
                "executor": task["executor"],
                "start": task["start"],
                "end": task["end"],
                "active_days": task["active-days"],
                "status": task["status"],
                "critical": task["critical"],
            },
        })

    links = [
        {"id": i + 1, "source": src, "target": dst, "type": "0"}
        for i, (src, dst) in enumerate(edges)
    ]

    js = DHTMLX_INIT_JS % {"start": window_start, "end": window_end, "data": embed_json({"data": data, "links": links})}
    css = inline_vendor((DHTMLX_PKG / "dhtmlxgantt.css").read_text(encoding="utf-8"))
    lib = inline_vendor((DHTMLX_PKG / "dhtmlxgantt.js").read_text(encoding="utf-8"))

    return f"""<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>schedule gantt — dhtmlx-gantt 10.0.3</title>
<style>
{css}
</style>
<style>
{DHTMLX_CUSTOM_CSS}
</style>
</head>
<body>
<div id="gantt-here"></div>
<script>
{lib}
</script>
<script>
{js}
</script>
</body>
</html>
"""


def main():
    scalars, dated, executors, edges, critical = build_model()
    check_model(scalars, dated, executors, edges, critical)

    FRAPPE_OUT.write_text(frappe_html(scalars, dated, executors, edges), encoding="utf-8")
    DHTMLX_OUT.write_text(dhtmlx_html(scalars, dated, executors, edges), encoding="utf-8")

    print(f"dated tasks: {len(dated)}, executors: {len(executors)}, arrows (both dated): {len(edges)}")
    print(f"wrote {FRAPPE_OUT.name} ({FRAPPE_OUT.stat().st_size} bytes)")
    print(f"wrote {DHTMLX_OUT.name} ({DHTMLX_OUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
