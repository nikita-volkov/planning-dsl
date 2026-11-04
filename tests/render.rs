//! The renderers: golden output for the canonical example, the determinism
//! guarantee (same plan in, byte-identical file out — including across
//! authoring order), and the self-containment of the HTML.

use planning_dsl::{load, render};

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!("fixtures/", $name))
    };
}

const PLAN: &str = fixture!("plan.yaml");

#[test]
fn the_canonical_example_renders_the_expected_mermaid() {
    let plan = load(PLAN).expect("parses");
    assert_eq!(
        render::mermaid(&plan),
        "flowchart TD\n\
         \x20   design-architecture[\"design-architecture\"]\n\
         \x20   plan-out-the-tasks[\"plan-out-the-tasks\"]\n\
         \x20   implement-services[\"implement-services\"]\n\
         \x20   implement-tests[\"implement-tests\"]\n\
         \x20   design-architecture --> plan-out-the-tasks\n\
         \x20   plan-out-the-tasks --> implement-services\n\
         \x20   plan-out-the-tasks --> implement-tests\n"
    );
}

#[test]
fn mermaid_edges_point_from_a_dependency_to_its_dependent() {
    // The community convention: arrows follow execution order.
    let plan = load(PLAN).expect("parses");
    let mmd = render::mermaid(&plan);
    assert!(
        mmd.contains("    design-architecture --> plan-out-the-tasks\n"),
        "{mmd}"
    );
    assert!(
        !mmd.contains("plan-out-the-tasks --> design-architecture"),
        "{mmd}"
    );
}

#[test]
fn authoring_order_is_meaningful_so_renders_follow_declaration_order() {
    // Declaration order is load-bearing (the scheduler's tie-break), so two
    // files that declare the same tasks in different orders are different
    // plans: their renders may order nodes differently but must carry the
    // same nodes and edges, and each is byte-identical across runs.
    let a = "
unit: person-hours
deadline: 2026-12-18
roles:
  dev: implements
tasks:
  ship: { role: dev, depends-on: [build, test] }
  test: { role: dev, estimate: 2 }
  build: { role: dev, estimate: 3 }
executors:
  anton: { roles: [dev] }
";
    let b = "
unit: person-hours
deadline: 2026-12-18
roles:
  dev: implements
tasks:
  build: { role: dev, estimate: 3 }
  ship: { role: dev, depends-on: [test, build] }
  test: { role: dev, estimate: 2 }
executors:
  anton: { roles: [dev] }
";
    let a = load(a).expect("parses");
    let b = load(b).expect("parses");
    // Mermaid: same lines up to order. HTML: geometry differs with the row
    // order, so compare the carried data instead — the node ids and the
    // edge pairs.
    let sorted = |s: &str| -> Vec<String> {
        let mut lines: Vec<String> = s.lines().map(str::to_string).collect();
        lines.sort();
        lines
    };
    assert_eq!(sorted(&render::mermaid(&a)), sorted(&render::mermaid(&b)));
    let html_data = |html: &str| -> (Vec<String>, Vec<String>) {
        let mut ids: Vec<String> = html
            .lines()
            .filter(|l| l.contains("data-id="))
            .filter_map(|l| l.split("data-id=\"").nth(1))
            .filter_map(|rest| rest.split('"').next())
            .map(str::to_string)
            .collect();
        ids.sort();
        let mut edges: Vec<String> = html
            .lines()
            .filter(|l| l.contains("class=\"edge\""))
            .filter_map(|l| {
                let from = l.split("data-from=\"").nth(1)?.split('"').next()?;
                let to = l.split("data-to=\"").nth(1)?.split('"').next()?;
                Some(format!("{from}->{to}"))
            })
            .collect();
        edges.sort();
        (ids, edges)
    };
    assert_eq!(html_data(&render::html(&a)), html_data(&render::html(&b)));
    // Determinism per plan, run over run.
    assert_eq!(render::mermaid(&a), render::mermaid(&a));
    assert_eq!(render::html(&b), render::html(&b));
    // The node order follows the declaration order of each file.
    let a_mermaid = render::mermaid(&a);
    let a_nodes: Vec<&str> = a_mermaid
        .lines()
        .filter(|l| l.contains("[\""))
        .map(|l| l.split('"').nth(1).expect("quoted"))
        .collect();
    assert_eq!(a_nodes, ["ship", "test", "build"]);
}

#[test]
fn a_task_name_mermaid_cannot_use_pushes_every_node_to_synthetic_ids() {
    // `end` breaks mermaid flowcharts; the whole diagram falls back to t0…tN.
    let yaml = "
unit: person-hours
deadline: 2026-12-18
roles:
  dev: implements
tasks:
  end: { role: dev }
  start: { role: dev, depends-on: [end] }
executors:
  anton: { roles: [dev] }
";
    let plan = load(yaml).expect("parses");
    let mmd = render::mermaid(&plan);
    assert!(mmd.contains("t0[\"end\"]"), "{mmd}");
    assert!(mmd.contains("    t0 --> t1\n"), "{mmd}");
    assert!(!mmd.contains("end["), "{mmd}");
}

#[test]
fn labels_are_escaped_in_both_renders() {
    let yaml = "
unit: person-hours
deadline: 2026-12-18
roles:
  dev: implements
tasks:
  fix&ship: { role: dev }
  a<b: { role: dev }
executors:
  anton: { roles: [dev] }
";
    let plan = load(yaml).expect("parses");
    let mmd = render::mermaid(&plan);
    assert!(mmd.contains("t0[\"fix&amp;ship\"]"), "{mmd}");
    assert!(mmd.contains("a&lt;b"), "{mmd}");
    let html = render::html(&plan);
    assert!(html.contains("fix&amp;ship"), "{html}");
    assert!(!html.contains("fix&ship"), "{html}");
    assert!(!html.contains("a<b"), "{html}");
}

#[test]
fn the_html_is_a_self_contained_file() {
    let plan = load(PLAN).expect("parses");
    let html = render::html(&plan);
    assert!(
        html.contains("<svg xmlns=\"http://www.w3.org/2000/svg\""),
        "{html}"
    );
    assert!(html.contains("<script>"), "{html}");
    assert!(!html.contains("<script src="), "{html}");
    assert!(!html.contains("<link "), "{html}");
    assert!(!html.contains("https://"), "{html}");
    assert_eq!(
        html.matches("http://").count(),
        1,
        "the only URL is the SVG xmlns"
    );
}

#[test]
fn every_task_is_one_node_and_every_dependency_one_edge() {
    let plan = load(PLAN).expect("parses");
    let html = render::html(&plan);
    assert_eq!(html.matches("class=\"node\"").count(), 4, "{html}");
    assert_eq!(html.matches("class=\"edge\"").count(), 3, "{html}");
    assert!(
        html.contains("data-from=\"plan-out-the-tasks\" data-to=\"implement-services\""),
        "{html}"
    );
    for name in [
        "design-architecture",
        "plan-out-the-tasks",
        "implement-services",
        "implement-tests",
    ] {
        assert!(html.contains(&format!("data-id=\"{name}\"")), "{html}");
    }
}

#[test]
fn a_dependency_naming_no_task_is_skipped_not_rendered() {
    let yaml = "
unit: person-hours
deadline: 2026-12-18
roles:
  dev: implements
tasks:
  ship: { role: dev, depends-on: [ghost, build] }
  build: { role: dev }
executors:
  anton: { roles: [dev] }
";
    let plan = load(yaml).expect("parses");
    let mmd = render::mermaid(&plan);
    assert!(!mmd.contains("ghost"), "{mmd}");
    let html = render::html(&plan);
    assert!(!html.contains("ghost"), "{html}");
    assert_eq!(html.matches("class=\"node\"").count(), 2, "{html}");
    assert_eq!(html.matches("class=\"edge\"").count(), 1, "{html}");
}
