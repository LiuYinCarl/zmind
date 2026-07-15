use std::env;

pub struct Cli;

impl Cli {
    pub fn read() {
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            if arg == "--version" {
                println!("zmind {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            } else if arg == "--help" {
                println!("zmind - A terminal-based mind mapping tool");
                println!("Usage: zmind");
                println!();
                println!("  --version   Print version and exit");
                println!("  --help      Print this help and exit");
                std::process::exit(0);
            }
        }
    }
}
