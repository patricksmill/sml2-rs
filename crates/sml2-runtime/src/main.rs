//! Command-line entry point for the reusable runtime.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    sml2_runtime::run_cli()
}
