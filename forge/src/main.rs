use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "forge", version, about = "Dwarf platform CLI — build, manage dependencies, and more")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    // Subcommands will be added in later chunks
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        None => {
            eprintln!("Error: No subcommand provided. Use --help for usage.");
            std::process::exit(1);
        }
        _ => {
            eprintln!("No subcommands implemented yet.");
            std::process::exit(1);
        }
    }
}
