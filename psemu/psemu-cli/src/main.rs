use clap::Parser;
use tracing::instrument;

use psemu_core::Cpu;
use psemudb::Debugger;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start in TUI debug mode
    #[arg(long)]
    debug_mode: bool,
    //    /// Number of times to greet
    //    #[arg(short, long, default_value_t = 1)]
    //    count: u8,
}

#[instrument]
fn main() {
    let args = Args::parse();
    tracing_subscriber::fmt::init();
    if !args.debug_mode {
        let mut cpu = Cpu::new();
        loop {
            cpu.run_single_cycle();
        }
    } else {
        let mut debugger = Debugger::new();
        debugger.run();
    }
}
