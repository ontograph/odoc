// Prints Rust std char properties for each code point read from stdin (one decimal per line).
use std::io::{self, BufRead, Write};
fn main() {
    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(n) = line.trim().parse::<u32>() else { continue };
        let Some(c) = char::from_u32(n) else { continue };
        let lower: String = c.to_lowercase().collect();
        let codes: Vec<String> = lower.chars().map(|l| (l as u32).to_string()).collect();
        let _ = writeln!(out, "{}\t{}\t{}\t{}\t{}\t{}", n, c.is_alphanumeric() as u8, c.is_uppercase() as u8,
            c.is_lowercase() as u8, c.is_numeric() as u8, codes.join(","));
    }
}
