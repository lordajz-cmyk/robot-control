//! `robotctl` — litet CLI-verktyg på Pi:n.
//!
//! **Arkitekturändring 2026-09-19:** Sedan robotd bytte till att lyssna
//! direkt över er WireGuard-VPN (PROJECT_SPEC.md §14) behövs ingen
//! kryptografisk parkoppling längre — att bli en godkänd WireGuard-peer
//! (era befintliga `wireguard.sh`/`wireguard_admin.sh`-skript) ÄR
//! parkopplingen. `robotctl pair` nedan är därför inte längre den
//! primära vägen in.
//!
//! Kvar att göra här: en riktig `view-code`-kommando som pratar med den
//! lokalt körande `robotd` (t.ex. via ett Unix-socket) för att generera en
//! tillfällig titta-kod utan att behöva vara inne i klientappen. Just nu
//! skriver den bara ut en kod på skärmen utan att faktiskt registrera den
//! hos robotd — TODO.

use clap::{Parser, Subcommand};
use rand::Rng;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generera en ny parkopplingskod (giltig 5 minuter) för en förar-dator.
    Pair,
    /// Generera en tillfällig titta-kod (giltig 30 minuter, bara läsrättighet).
    ViewCode,
    /// Visa robotens namn/ID och publika nyckel.
    Status,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Pair => {
            let code = random_code();
            println!("Parkopplingskod (giltig 5 min): {code}");
            // TODO: skicka RegisterPairingCode till lokalt körande robotd,
            // t.ex. via ett lokalt Unix-socket eller HTTP på localhost.
        }
        Command::ViewCode => {
            let code = random_code();
            println!("Titta-kod (giltig 30 min, ingen körrätt): {code}");
        }
        Command::Status => {
            println!("(TODO: läs status från lokalt körande robotd)");
        }
    }
}

fn random_code() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // undviker lätt förväxlade tecken
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}
