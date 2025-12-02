use clap::Parser;
use std::io::{self, Read};
use anyhow::Result;

#[derive(Parser, Debug)]
#[command(name = "emasm")]
#[command(about = "EVM Assembler CLI", long_about = None)]
struct Args {
    /// Input file (use - for stdin)
    #[arg(default_value = "-")]
    input: String,
    
    /// Output format: hex, bin, or both
    #[arg(short, long, default_value = "hex")]
    format: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    let input = if args.input == "-" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else {
        std::fs::read_to_string(&args.input)?
    };
    
    println!("emasm-cli: Assembly functionality coming soon!");
    println!("Input length: {} bytes", input.len());
    
    Ok(())
}
