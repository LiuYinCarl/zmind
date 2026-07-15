use std::{env, path::PathBuf};

pub struct Cli {
    pub filename: Option<PathBuf>,
}

impl Cli {
    pub fn read() -> Self {
        let mut args = env::args().skip(1);

        let mut filename: Option<PathBuf> = None;

        while let Some(arg) = args.next() {
            if arg == "--version" {
                println!("zmind {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            } else if arg == "--help" {
                println!("zmind - A terminal-based mind mapping tool");
                println!("Usage: zmind [filename]");
                println!();
                println!("  filename    Optional path to a .json mind map file");
                println!("  --version   Print version and exit");
                println!("  --help      Print this help and exit");
                std::process::exit(0);
            } else if !arg.starts_with('-') {
                filename = Some(PathBuf::from(arg));
            }
        }

        Cli { filename }
    }
}
