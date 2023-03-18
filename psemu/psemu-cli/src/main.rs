use std::{
    io::Write,
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
};

use clap::Parser;

use psemu_core::Cpu;
use psemudb::Debugger;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start in TUI debug mode
    #[arg(long)]
    debug_mode: bool,
    /// Step through instructions automatically
    #[arg(long, default_value_t = true)]
    auto: bool,
    //    /// Number of times to greet
    //    #[arg(short, long, default_value_t = 1)]
    //    count: u8,
}

struct ChannelLogger {
    tx: &'static mut Sender<String>,
}

impl ChannelLogger {
    pub fn new(logs: Arc<Mutex<Vec<String>>>) -> Self {
        let (tx, rx) = mpsc::channel();
        // TODO: See if there's a better way to prevent `tx` from getting dropped
        let tx = Box::leak(Box::new(tx));
        let logs_clone = logs;
        tokio::spawn(async move {
            loop {
                match rx.recv() {
                    Ok(msg) => {
                        if let Ok(mut logs) = logs_clone.lock() {
                            logs.push(msg)
                        }
                    }
                    Err(e) => {
                        println!("Could not receive: {e}")
                    }
                }
            }
        });
        ChannelLogger { tx }
    }
}

impl Write for ChannelLogger {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = String::from_utf8_lossy(buf).to_string();
        self.tx.send(s).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if !args.debug_mode {
        tracing_subscriber::fmt::init();
        let mut cpu = Cpu::new();
        loop {
            cpu.run_single_cycle();
        }
    } else {
        let logs = Arc::new(Mutex::new(vec![]));
        let chan_logger = ChannelLogger::new(logs.clone());
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_writer(Mutex::new(chan_logger))
            .finish();
        let _default = tracing::subscriber::set_default(subscriber);

        let mut debugger = Debugger::new(logs, args.auto);
        debugger.run();
    }
}
