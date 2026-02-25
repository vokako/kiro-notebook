#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mcp-server") {
        kiro_notebook_lib::run_mcp();
    } else {
        kiro_notebook_lib::run();
    }
}
