use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: planning-dsl <command>

commands:
  validate <plan.yaml>   check a plan; every error, not just the first
  schedule <plan.yaml>   compute the schedule; print the artifact
      --out <file>       write it to a file instead of stdout
                         (never into plan.yaml; no date flags — what-ifs
                         are file edits under git)
  emit-schema            print the JSON Schema for plan.yaml
  render <plan.yaml>     write the derived renders beside the plan
      --mmd <file>       the mermaid flowchart (default: <stem>.mmd)
      --html <file>      the interactive graph (default: <stem>.html)
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
    let out = planning_dsl::schedule::to_yaml(&artifact);
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
    // Everything is computed before anything is written: the gantt runs the
    // scheduler, and a refusal must leave no half-drawn set behind.
    let mut outputs: Vec<(&Path, String)> = vec![
        (mmd_out, planning_dsl::render::mermaid(&plan)),
        (html_out, planning_dsl::render::html(&plan)),
    ];
    if let Some(named_gantt) = gantt_path {
        let artifact = match planning_dsl::schedule(&plan) {
            Ok(artifact) => artifact,
            Err(refusal) => {
                eprintln!("{refusal}");
                return ExitCode::FAILURE;
            }
        };
        let gantt_out: &Path = match named_gantt {
            Some(p) => Path::new(p),
            None => default_gantt.as_path(),
        };
        outputs.push((gantt_out, planning_dsl::render::gantt(&artifact)));
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
