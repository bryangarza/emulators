use std::{
    io::Write,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
};

use clap::Parser;
use tracing::instrument::{self, WithSubscriber};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

struct ChannelLogger {
    tx: Sender<String>,
}

impl ChannelLogger {
    pub fn new(logs: Arc<Mutex<Vec<String>>>) -> Self {
        let (tx, rx) = mpsc::channel();
        let logs_clone = logs.clone();
        tokio::spawn(async move {
            loop {
                let msg = rx.recv().unwrap();
                if let Ok(mut logs) = logs_clone.lock() {
                    logs.push(msg)
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

// #[instrument]
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
            .with_writer(Mutex::new(chan_logger))
            .finish();
        let _default = tracing::subscriber::set_default(subscriber);
        // tracing_subscriber::fmt::init().with_subscriber(subscriber);

        // tracing_subscriber::registry()
        //   .with(tui_logger::tracing_subscriber_layer())
        //   .init();
        let mut debugger = Debugger::new(logs);
        debugger.run();
    }
}
