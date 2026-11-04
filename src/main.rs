use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: planning-dsl <command>

commands:
  validate <plan.yaml>   check a plan; every error, not just the first
  schedule <plan.yaml>   compute the schedule; print the artifact
      --out <file>       write it to a file instead of stdout
      --format <fmt>     yaml (the default), csv or tsv — csv/tsv are one
                         flat table: the plan columns repeat on every task
                         row, the critical path becomes on-critical-path
                         (never into plan.yaml; no date flags — what-ifs
                         are file edits under git)
  emit-schema            print the JSON Schema for plan.yaml
  render <plan.yaml>     write the derived renders beside the plan
      --mmd <file>       the mermaid flowchart (default: <stem>.mmd)
      --html <file>      the interactive graph (default: <stem>.html); on a
                         plan that schedules it marks the critical path — a
                         scheduling refusal warns, the graph still writes
      --direction <dir>  graph layout: tb (top-down, the default) or lr
                         (left-to-right)
      --gantt [<file>]   the mermaid gantt (default: <stem>.gantt.mmd);
                         opt-in: it runs the scheduler, so the plan needs a
                         start: and full estimates — a refusal writes nothing
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("validate") => validate(&args[2..]),
        Some("schedule") => schedule(&args[2..]),
        Some("render") => render(&args[2..]),
        Some("emit-schema") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&planning_dsl::json_schema())
                    .expect("schema serializes")
            );
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn validate(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("error: validate needs a plan file\n{USAGE}");
        return ExitCode::from(2);
    };
    let yaml = match std::fs::read_to_string(path) {
        Ok(yaml) => yaml,
        Err(e) => {
            eprintln!("error: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    match planning_dsl::check(&yaml) {
        Err(parse_error) => {
            eprintln!("{path}: {parse_error}");
            ExitCode::FAILURE
        }
        Ok(report) => {
            for warning in &report.warnings {
                eprintln!("warning: {warning}");
            }
            if report.is_ok() {
                println!("{path}: ok");
                ExitCode::SUCCESS
            } else {
                for error in &report.errors {
                    eprintln!("error: {error}");
                }
                eprintln!(
                    "{path}: {} error{}",
                    report.errors.len(),
                    if report.errors.len() == 1 { "" } else { "s" }
                );
                ExitCode::FAILURE
            }
        }
    }
}

fn schedule(args: &[String]) -> ExitCode {
    let mut plan_path = None;
    let mut out_path = None;
    // yaml unless asked otherwise; validated against the three spellings.
    let mut format = "yaml";
    let mut flags = args.iter();
    while let Some(arg) = flags.next() {
        match arg.as_str() {
            "--out" => {
                let Some(target) = flags.next() else {
                    eprintln!("error: --out needs a file\n{USAGE}");
                    return ExitCode::from(2);
                };
                out_path = Some(target);
            }
            "--format" => {
                let Some(value) = flags.next() else {
                    eprintln!("error: --format needs yaml, csv or tsv\n{USAGE}");
                    return ExitCode::from(2);
                };
                format = match value.as_str() {
                    "yaml" | "csv" | "tsv" => value.as_str(),
                    other => {
                        eprintln!("error: --format is yaml, csv or tsv, not {other}\n{USAGE}");
                        return ExitCode::from(2);
                    }
                };
            }
            s if s.starts_with('-') => {
                eprintln!("error: unknown flag {s}\n{USAGE}");
                return ExitCode::from(2);
            }
            _ => {
                if plan_path.is_some() {
                    eprintln!("error: schedule takes one plan file\n{USAGE}");
                    return ExitCode::from(2);
                }
                plan_path = Some(arg);
            }
        }
    }
    let Some(path) = plan_path else {
        eprintln!("error: schedule needs a plan file\n{USAGE}");
        return ExitCode::from(2);
    };
    let yaml = match std::fs::read_to_string(path) {
        Ok(yaml) => yaml,
        Err(e) => {
            eprintln!("error: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let plan = match planning_dsl::load(&yaml) {
        Ok(plan) => plan,
        Err(parse_error) => {
            eprintln!("{path}: {parse_error}");
            return ExitCode::FAILURE;
        }
    };
    let report = planning_dsl::validate(&plan);
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    if !report.is_ok() {
        for error in &report.errors {
            eprintln!("error: {error}");
        }
        eprintln!("{path}: not scheduled — fix the errors above first");
        return ExitCode::FAILURE;
    }
    let artifact = match planning_dsl::schedule(&plan) {
        Ok(artifact) => artifact,
        Err(refusal) => {
            eprintln!("{refusal}");
            return ExitCode::FAILURE;
        }
    };
    let out = match format {
        "csv" => planning_dsl::schedule::to_csv(&artifact),
        "tsv" => planning_dsl::schedule::to_tsv(&artifact),
        _ => planning_dsl::schedule::to_yaml(&artifact),
    };
    match out_path {
        Some(file) => {
            if let Err(e) = std::fs::write(file, &out) {
                eprintln!("error: cannot write {file}: {e}");
                return ExitCode::from(2);
            }
            println!("{path}: wrote {file}");
        }
        None => print!("{out}"),
    }
    ExitCode::SUCCESS
}

fn render(args: &[String]) -> ExitCode {
    let mut plan_path = None;
    let mut mmd_path = None;
    let mut html_path = None;
    // The html graph's flow direction; top-down unless asked otherwise.
    let mut direction = planning_dsl::render::Direction::Tb;
    // Outer `Some` = the gantt was requested; inner = the file was named.
    let mut gantt_path: Option<Option<&str>> = None;
    let mut flags = args.iter().peekable();
    while let Some(arg) = flags.next() {
        match arg.as_str() {
            "--mmd" | "--html" | "--gantt" => {
                // A following non-flag token is the file name. `--gantt` may
                // also stand alone: its presence, not a name, is the
                // request — unnamed, it takes the default file.
                let named = match flags.peek() {
                    Some(next) if !next.starts_with('-') => flags.next(),
                    _ => None,
                };
                let target = match named {
                    Some(v) => Some(v.as_str()),
                    None if arg == "--gantt" => None,
                    None => {
                        eprintln!("error: {arg} needs a file\n{USAGE}");
                        return ExitCode::from(2);
                    }
                };
                match arg.as_str() {
                    "--mmd" => mmd_path = target,
                    "--html" => html_path = target,
                    _ => gantt_path = Some(target),
                }
            }
            "--direction" => {
                let Some(value) = flags.next() else {
                    eprintln!("error: --direction needs tb or lr\n{USAGE}");
                    return ExitCode::from(2);
                };
                direction = match value.as_str() {
                    "tb" => planning_dsl::render::Direction::Tb,
                    "lr" => planning_dsl::render::Direction::Lr,
                    other => {
                        eprintln!("error: --direction is tb or lr, not {other}\n{USAGE}");
                        return ExitCode::from(2);
                    }
                };
            }
            s if s.starts_with('-') => {
                eprintln!("error: unknown flag {s}\n{USAGE}");
                return ExitCode::from(2);
            }
            _ => {
                if plan_path.is_some() {
                    eprintln!("error: render takes one plan file\n{USAGE}");
                    return ExitCode::from(2);
                }
                plan_path = Some(arg);
            }
        }
    }
    let Some(path) = plan_path else {
        eprintln!("error: render needs a plan file\n{USAGE}");
        return ExitCode::from(2);
    };
    let yaml = match std::fs::read_to_string(path) {
        Ok(yaml) => yaml,
        Err(e) => {
            eprintln!("error: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let plan = match planning_dsl::load(&yaml) {
        Ok(plan) => plan,
        Err(parse_error) => {
            eprintln!("{path}: {parse_error}");
            return ExitCode::FAILURE;
        }
    };
    let report = planning_dsl::validate(&plan);
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    if !report.is_ok() {
        for error in &report.errors {
            eprintln!("error: {error}");
        }
        eprintln!("{path}: not rendered — fix the errors above first");
        return ExitCode::FAILURE;
    }
    let default_mmd = Path::new(path).with_extension("mmd");
    let default_html = Path::new(path).with_extension("html");
    let default_gantt = Path::new(path).with_extension("gantt.mmd");
    let mmd_out: &Path = match mmd_path {
        Some(p) => Path::new(p),
        None => default_mmd.as_path(),
    };
    let html_out: &Path = match html_path {
        Some(p) => Path::new(p),
        None => default_html.as_path(),
    };
    // The schedule, once, up front — both consumers read the one artifact.
    // Best-effort for the html graph: the critical path is an enhancement,
    // and the graph is most useful on early drafts that cannot schedule
    // yet, so a refusal warns and the graph writes unmarked. The gantt
    // keeps its loud gate: a refusal aborts with nothing written —
    // everything is computed before anything is written.
    let scheduled = planning_dsl::schedule(&plan);
    let artifact = match scheduled.as_ref() {
        Ok(artifact) => Some(artifact),
        Err(refusal) => {
            if gantt_path.is_some() {
                eprintln!("{refusal}");
                return ExitCode::FAILURE;
            }
            // The refusal leads with `error: ` when it has a lead line of
            // its own; downgraded to a warning, the contradiction of
            // `warning: error: …` is not worth keeping.
            let lead = refusal.strip_prefix("error: ").unwrap_or(refusal);
            eprintln!("warning: {lead}");
            None
        }
    };
    let mut outputs: Vec<(&Path, String)> = vec![
        (mmd_out, planning_dsl::render::mermaid(&plan)),
        (
            html_out,
            planning_dsl::render::html(&plan, direction, artifact),
        ),
    ];
    if let Some(named_gantt) = gantt_path {
        let gantt_out: &Path = match named_gantt {
            Some(p) => Path::new(p),
            None => default_gantt.as_path(),
        };
        let artifact = artifact.expect("a gantt request gated on the schedule above");
        outputs.push((gantt_out, planning_dsl::render::gantt(artifact)));
    }
    for (out, content) in &outputs {
        if let Err(e) = std::fs::write(out, content) {
            eprintln!("error: cannot write {}: {e}", out.display());
            return ExitCode::from(2);
        }
    }
    let written = outputs
        .iter()
        .map(|(out, _)| out.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    println!("{path}: wrote {written}");
    ExitCode::SUCCESS
}
