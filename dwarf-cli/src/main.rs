//! CLI entry point for the Dwarf compiler.

use clap::Parser;
use dwarf_cli::{build, check, dev, emit, fmt, install, run, test, Cli, Commands};

fn main() {
    let cli = Cli::parse();

    if cli.list_runtimes {
        for rt in dwarf_cli::runner::list_runtimes() {
            println!("{}", rt);
        }
        return;
    }

    match cli.command {
        Some(Commands::Run {
            files,
            target,
            passes,
            skip_passes,
            stdlib_path,
        }) => {
            run::run_run(files, target, passes, skip_passes, stdlib_path);
        }
        Some(Commands::Check {
            files,
            json,
            passes,
            skip_passes,
            list_passes,
            stdlib_path,
        }) => {
            check::run_check(files, json, passes, skip_passes, list_passes, stdlib_path);
        }
        Some(Commands::Emit {
            files,
            target,
            json,
            passes,
            skip_passes,
            stdlib_path,
        }) => {
            emit::run_emit(files, target, json, passes, skip_passes, stdlib_path);
        }
        Some(Commands::Dev {
            files,
            target,
            passes,
            skip_passes,
            stdlib_path,
        }) => {
            dev::run_dev(files, target, passes, skip_passes, stdlib_path);
        }
        Some(Commands::Build {
            files,
            target,
            out_dir,
            pretty,
            source_map,
            json,
            passes,
            skip_passes,
            stdlib_path,
        }) => {
            build::run_build(
                files,
                target,
                out_dir,
                pretty,
                source_map,
                json,
                passes,
                skip_passes,
                stdlib_path,
            );
        }
        Some(Commands::Fmt {
            files,
            check,
            stdout,
        }) => {
            fmt::run_fmt(files, check, stdout);
        }
        Some(Commands::Test {
            files,
            target,
            json,
            diff,
            fix,
        }) => {
            test::run_test(files, target, json, diff, fix);
        }
        Some(Commands::Install { package }) => {
            install::run_install(&package);
        }
        None => {
            eprintln!("Error: No subcommand provided. Use --help for usage.");
            std::process::exit(1);
        }
    }
}
