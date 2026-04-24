mod analyze;
mod domain;
mod fix;
mod model;
mod qemu;
mod trace;
mod viz;

use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::thread;
use std::time::Duration;

use domain::Domain;
use fix::suggest_fix;
use model::TraceEvent;
use qemu::qmp::QmpClient;
use trace::symbol::SymbolResolver;
use viz::timeline::run_tui;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Trace {
        file: String,
    },
    Live {
        #[arg(short, long, default_value = "./qmp.sock")]
        socket: String,
    },
    Analyze {
        file: String,
        #[arg(long)]
        json: bool,
    },
    Panic {
        file: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        no_viz: bool,
    },
}

fn detect_anomalies(events: &[TraceEvent]) {
    for window in events.windows(2) {
        let prev = &window[0];
        let curr = &window[1];

        if prev.domain != curr.domain && prev.pkru == curr.pkru {
            println!(
                "Anomaly at TSC {}: invalid domain transition ({:?} -> {:?} without PKRU update)",
                curr.tsc, prev.domain, curr.domain
            );
        }
        if prev.domain == curr.domain && prev.pkru != curr.pkru {
            println!(
                "Anomaly at TSC {}: PKRU change without domain change ({:?} PKRU 0x{:x} -> 0x{:x})",
                curr.tsc, prev.domain, prev.pkru, curr.pkru
            );
        }
        if curr.rip == 0 {
            println!("Anomaly at TSC {}: sudden RIP = 0x0", curr.tsc);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let bin_path = "./target/x86_64-sasos/debug/sexos";
    let mut resolver = SymbolResolver::new(bin_path);

    match &cli.command {
        Commands::Trace { file } => {
            let f = File::open(file)?;
            let reader = BufReader::new(f);

            let mut events = Vec::new();
            for line in reader.lines() {
                if let Ok(l) = line {
                    if let Some(mut event) = trace::parser::parse_line(&l) {
                        event.symbol = Some(resolver.resolve(event.rip));
                        events.push(event);
                    }
                }
            }

            detect_anomalies(&events);

            println!("Press enter to start UI...");
            let mut s = String::new();
            std::io::stdin().read_line(&mut s)?;

            run_tui(&events, None)?;
        }
        Commands::Live { socket } => {
            let mut client = QmpClient::connect(socket)?;
            let mut events = Vec::new();
            let mut tsc = 0;

            println!("Polling QMP...");
            for _ in 0..100 {
                if let Ok(Some((rip, pkru, _cr3))) = client.info_registers() {
                    let mut event = TraceEvent {
                        tsc,
                        rip,
                        pkru,
                        domain: Domain::from_pkru(pkru),
                        symbol: None,
                    };
                    event.symbol = Some(resolver.resolve(rip));
                    events.push(event);
                    tsc += 1;
                }
                thread::sleep(Duration::from_millis(100));
            }

            detect_anomalies(&events);
            run_tui(&events, None)?;
        }
        Commands::Analyze { file, json } => {
            let f = File::open(file)?;
            let reader = BufReader::new(f);

            let mut events = Vec::new();
            for line in reader.lines() {
                if let Ok(l) = line {
                    if let Some(event) = trace::parser::parse_line(&l) {
                        events.push(event);
                    }
                }
            }

            let mut result = analyze::engine::analyze(&events);
            if !events.is_empty() {
                if let Some(event) = events.get(result.first_bad_index) {
                    let sym = resolver.resolve(event.rip);
                    if sym.starts_with("0x") || sym == "??" {
                        result.function = None;
                    } else {
                        result.function = Some(sym);
                    }
                }
            }

            if *json {
                let report = analyze::report::build_report(&result, &events);
                let output = serde_json::to_string_pretty(&report)
                    .map_err(|e| format!("json error: {}", e))?;
                println!("{}", output);
                return Ok(());
            }

            analyze::report::print_report(&result, &events);
        }
        Commands::Panic { file, json, no_viz } => {
            let f = File::open(file)?;
            let reader = BufReader::new(f);

            let mut events = Vec::new();
            for line in reader.lines() {
                if let Ok(l) = line {
                    if let Some(mut event) = trace::parser::parse_line(&l) {
                        event.symbol = Some(resolver.resolve(event.rip));
                        events.push(event);
                    }
                }
            }

            if events.is_empty() {
                if *json {
                    println!(r#"{{"error": "Empty trace", "analysis": null, "fix": null}}"#);
                } else {
                    println!("=== SEX-DEBUG PANIC REPORT ===\n");
                    println!("Minimal Report: Trace file is empty. No events to analyze.");
                }
                return Ok(());
            }

            let mut analysis_result = analyze::engine::analyze(&events);
            if let Some(event) = events.get(analysis_result.first_bad_index) {
                let sym = resolver.resolve(event.rip);
                if sym.starts_with("0x") || sym == "??" {
                    analysis_result.function = None;
                } else {
                    analysis_result.function = Some(sym);
                }
            }
            
            let suggestion = suggest_fix(&analysis_result);

            if *json {
                let output = serde_json::json!({
                    "analysis": {
                        "root_cause": analysis_result.root_cause,
                        "confidence": analysis_result.confidence,
                        "location": analysis_result.function.unwrap_or_else(|| "Unknown".to_string()),
                        "failing_index": analysis_result.first_bad_index
                    },
                    "fix": {
                        "message": suggestion.message
                    }
                });
                println!("{}", serde_json::to_string_pretty(&output).map_err(|e| format!("json error: {}", e))?);
            } else {
                println!("=== SEX-DEBUG PANIC REPORT ===\n");
                println!("Root Cause:");
                println!("{}\n", analysis_result.root_cause);
                println!("Confidence:");
                println!("{:.2}", analysis_result.confidence);
                if analysis_result.confidence < 0.5 {
                    println!("⚠️ Warning: Low confidence diagnosis");
                }
                println!();
                println!("Location:");
                let loc = analysis_result.function.clone().unwrap_or_else(|| "Unknown".to_string());
                println!("{}\n", loc);
                println!("Suggested Fix:");
                println!("{}\n", suggestion.message);
            }

            if !no_viz {
                run_tui(&events, Some(analysis_result.first_bad_index))?;
            }
        }
    }

    Ok(())
}
