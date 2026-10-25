//! One UTF-8 compilation unit on stdin; versioned JSON records on stdout.
mod attributes;
mod owners;
mod score;

use std::io::{self, Read};
use syn::visit::Visit;

fn analyze(source: &str) -> Result<serde_json::Value, syn::Error> {
    let file = syn::parse_file(source)?;
    let bom = source.len() - source.strip_prefix('\u{feff}').unwrap_or(source).len();
    let shebang = file.shebang.as_ref().map_or(0, String::len);
    let mut owners = owners::Owners::new(bom + shebang);
    owners.visit_file(&file);
    Ok(serde_json::json!({"version": 1, "records": owners.records}))
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut source = String::new();
    io::stdin().read_to_string(&mut source)?;
    println!("{}", analyze(&source)?);
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rust parse error: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
