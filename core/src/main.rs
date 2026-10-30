use merci_core::{
    game::Service,
    protocol::{Request, Response},
};
use std::{
    io::{BufRead, Write},
    path::PathBuf,
    sync::atomic::AtomicBool,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "Usage: merci-cli <data-directory> <Fairy-Stockfish.exe>\nReads one JSON command per line and returns one JSON response."
        );
        std::process::exit(2);
    }
    let mut service = Service::open(&PathBuf::from(&args[1]), &PathBuf::from(&args[2]))?;
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => match service.handle(&request, &AtomicBool::new(false)) {
                Ok(snapshot) => Response {
                    request_id: request.request_id,
                    ok: true,
                    snapshot: Some(snapshot),
                    error: String::new(),
                },
                Err(error) => Response {
                    request_id: request.request_id,
                    ok: false,
                    snapshot: service.snapshot().ok(),
                    error,
                },
            },
            Err(error) => Response {
                error: error.to_string(),
                ..Default::default()
            },
        };
        println!("{}", serde_json::to_string(&response)?);
        std::io::stdout().flush()?;
    }
    Ok(())
}
