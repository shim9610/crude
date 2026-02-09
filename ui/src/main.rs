// ui/src/main.rs
//! Cross-platform build tasks
//! 
//! Usage:
//!   cargo ui build      - Build browser_runner + work_flow_ui (debug)
//!   cargo ui run        - Run work_flow_ui after build (debug)
//!   cargo ui release    - Build release version

use std::process::{Command, exit};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let task = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    
    match task {
        "build" => build(false),
        "release" => build(true),
        "run" => run(false),
        "run-release" => run(true),
        "help" | "--help" | "-h" => help(),
        _ => {
            eprintln!("❌ Unknown command: {}", task);
            help();
            exit(1);
        }
    }
}

fn help() {
    println!("Usage: cargo ui <command>");
    println!();
    println!("Commands:");
    println!("  build        Build browser_runner + work_flow_ui (debug)");
    println!("  release      Build browser_runner + work_flow_ui (release)");
    println!("  run          Run work_flow_ui after build (debug)");
    println!("  run-release  Run work_flow_ui after build (release)");
    println!("  help         Show this help message");
}

fn build(release: bool) {
    let mode = if release { "release" } else { "debug" };
    println!("🔨 Building browser_runner ({})...", mode);
    
    if !cargo_build("browser_runner", release) {
        eprintln!("❌ browser_runner build failed");
        exit(1);
    }
    
    println!("🔨 Building work_flow_ui ({})...", mode);
    
    if !cargo_build("work_flow_ui", release) {
        eprintln!("❌ work_flow_ui build failed");
        exit(1);
    }
    
    println!("✅ Build complete!");
}

fn run(release: bool) {
    build(release);
    
    println!("🚀 Running work_flow_ui...");
    
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--bin", "work_flow_ui", "--features", "dev-tools"]);
    
    if release {
        cmd.arg("--release");
    }
    
    let status = cmd.status().expect("Execution failed");
    exit(status.code().unwrap_or(1));
}

fn cargo_build(bin: &str, release: bool) -> bool {
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--bin", bin, "--features", "dev-tools"]);
    
    if release {
        cmd.arg("--release");
    }
    
    cmd.status().map(|s| s.success()).unwrap_or(false)
}