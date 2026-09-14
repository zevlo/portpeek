mod gate;
mod parse;
mod scan;
mod services;
mod wait;

use std::io::{IsTerminal, Write};
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use scan::{PortResult, Status};

const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

#[derive(Parser, Debug)]
#[command(
    name = "portpeek",
    version,
    about = "A polite, localhost-first TCP port peeker",
    long_about = "portpeek answers one question: for these ports, is something listening?\n\
                  Open / closed / filtered, concurrent via tokio, honest about what it can't see.\n\
                  Scans 127.0.0.1 by default; scanning remote hosts requires --allow-remote."
)]
struct Cli {
    /// Target host [default: 127.0.0.1]
    host: String,

    /// Ports to peek: 80,443,1-1024
    port_spec: String,

    /// Allow scanning non-loopback hosts
    #[arg(long)]
    allow_remote: bool,

    /// Poll until all requested ports are open, give up after DUR
    #[arg(long, value_name = "DUR", value_parser = parse::parse_duration)]
    wait: Option<Duration>,

    /// Max concurrent connect() calls
    #[arg(long, default_value_t = 200, value_name = "N")]
    concurrency: usize,

    /// Per-port connect timeout
    #[arg(
        long,
        default_value = "1s",
        value_name = "DUR",
        value_parser = parse::parse_duration
    )]
    timeout: Duration,

    /// Only show open ports
    #[arg(long)]
    open_only: bool,
}

struct Style {
    enabled: bool,
}

impl Style {
    fn detect() -> Self {
        Style {
            enabled: std::io::stdout().is_terminal()
                && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty()),
        }
    }

    fn wrap(&self, code: &str, text: &str) -> String {
        if self.enabled {
            format!("{code}{text}{RESET}")
        } else {
            text.to_string()
        }
    }

    fn status(&self, status: Status) -> String {
        let code = match status {
            Status::Open => GREEN,
            Status::Closed => DIM,
            Status::Filtered => YELLOW,
        };
        self.wrap(code, &format!("{:<8}", status.label()))
    }
}

fn format_latency(d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{:.1}ms", secs * 1000.0)
    } else {
        format!("{secs:.1}s")
    }
}

fn print_results(results: &[PortResult], open_only: bool, style: &Style) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(
        out,
        "{:>5}  {:<12}  {:<8}  LATENCY",
        "PORT", "SERVICE", "STATUS"
    );
    for r in results {
        if open_only && r.status != Status::Open {
            continue;
        }
        let _ = writeln!(
            out,
            "{:>5}  {:<12}  {}  {}",
            r.port,
            services::name(r.port),
            style.status(r.status),
            format_latency(r.latency),
        );
    }
}

fn summary_line(results: &[PortResult], elapsed: Duration) -> String {
    let open = results.iter().filter(|r| r.status == Status::Open).count();
    let closed = results
        .iter()
        .filter(|r| r.status == Status::Closed)
        .count();
    let filtered = results
        .iter()
        .filter(|r| r.status == Status::Filtered)
        .count();
    format!(
        "{open} open, {closed} closed, {filtered} filtered in {:.1}s",
        elapsed.as_secs_f64()
    )
}

fn run(cli: &Cli, ports: &[u16]) -> (Vec<PortResult>, bool, Option<u32>) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");
    match cli.wait {
        Some(deadline) => {
            let outcome = rt.block_on(wait::wait_for_all(
                &cli.host,
                ports,
                cli.concurrency,
                cli.timeout,
                deadline,
            ));
            (outcome.results, outcome.all_open, Some(outcome.attempts))
        }
        None => {
            let results = rt.block_on(scan::scan(&cli.host, ports, cli.concurrency, cli.timeout));
            let any_open = results.iter().any(|r| r.status == Status::Open);
            (results, any_open, None)
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(msg) = gate::gate(&cli.host, cli.allow_remote) {
        eprintln!("error: {msg}");
        return ExitCode::from(2);
    }
    let ports = match parse::parse(&cli.port_spec) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    let started = std::time::Instant::now();
    let (results, ok, attempts) = run(&cli, &ports);
    let elapsed = started.elapsed();

    let style = Style::detect();
    print_results(&results, cli.open_only, &style);
    let summary = summary_line(&results, elapsed);
    if let Some(attempts) = attempts.filter(|_| !ok) {
        eprintln!(
            "{}",
            style.wrap(
                YELLOW,
                &format!("wait deadline expired after {attempts} attempts: {summary}")
            )
        );
    } else {
        println!("{}", style.wrap(BOLD, &summary));
    }

    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
