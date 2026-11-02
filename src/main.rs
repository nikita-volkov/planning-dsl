use std::process::ExitCode;

const USAGE: &str = "usage: planning-dsl <command>

commands:
  validate <plan.yaml>   check a plan; every error, not just the first
  emit-schema            print the JSON Schema for plan.yaml
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("validate") => validate(&args[2..]),
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
