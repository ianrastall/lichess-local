use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

pub const PROFILES: [(i32, u64, u32); 8] = [
    (-20, 100, 1),
    (-10, 300, 2),
    (-5, 1000, 3),
    (0, 3000, 5),
    (5, 10000, 7),
    (10, 30000, 10),
    (15, 100000, 14),
    (20, 300000, 18),
];

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn search(
    path: &Path,
    variant: &str,
    chess960: bool,
    fen: &str,
    moves: &[String],
    level: i32,
    cancel: &AtomicBool,
    analysis: bool,
) -> Result<(String, String), String> {
    if !(1..=8).contains(&level) {
        return Err("Unknown computer level".into());
    }
    let mut cmd = Command::new(path);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut process = Process(
        cmd.spawn()
            .map_err(|e| format!("Cannot start Fairy-Stockfish: {e}"))?,
    );
    let stdout = process.0.stdout.take().ok_or("Engine stdout unavailable")?;
    let mut input = process.0.stdin.take().ok_or("Engine stdin unavailable")?;
    let (tx, rx) = mpsc::sync_channel::<String>(256);
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else {
                break;
            };
            if line.len() > 16384 {
                break;
            }
            if line.starts_with("info ") {
                let _ = tx.try_send(line);
            } else if tx.send(line).is_err() {
                break;
            }
        }
    });
    let mut send = |line: &str| -> Result<(), String> {
        writeln!(input, "{line}")
            .and_then(|_| input.flush())
            .map_err(|e| e.to_string())
    };
    let wait = |expected: &str| -> Result<Vec<String>, String> {
        let until = Instant::now() + Duration::from_secs(15);
        let mut lines = Vec::new();
        loop {
            if cancel.load(Ordering::Acquire) {
                return Err("Cancelled".into());
            }
            if Instant::now() >= until {
                return Err("Fairy-Stockfish did not become ready".into());
            }
            match rx.recv_timeout(Duration::from_millis(30)) {
                Ok(line) => {
                    let done = line == expected;
                    lines.push(line);
                    if done {
                        return Ok(lines);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(_) => return Err("Fairy-Stockfish exited unexpectedly".into()),
            }
        }
    };
    let result = (|| {
        send("uci")?;
        let hello = wait("uciok")?;
        if !hello
            .iter()
            .any(|s| s.starts_with("id name Fairy-Stockfish"))
        {
            return Err("Unexpected engine identity".into());
        }
        let variant_options = hello
            .iter()
            .find(|s| s.starts_with("option name UCI_Variant "))
            .ok_or("Engine has no variant support")?;
        if !variant_options
            .split(" var ")
            .skip(1)
            .any(|s| s.split_whitespace().next() == Some(variant))
        {
            return Err(format!("Bundled engine does not support {variant}"));
        }
        let (skill, nodes, depth) = PROFILES[(level - 1) as usize];
        for line in [
            "setoption name Threads value 1".to_string(),
            "setoption name Hash value 32".to_string(),
            format!("setoption name UCI_Variant value {variant}"),
            format!("setoption name UCI_Chess960 value {chess960}"),
            format!(
                "setoption name Skill Level value {}",
                if analysis { 20 } else { skill }
            ),
            "ucinewgame".into(),
            "isready".into(),
        ] {
            send(&line)?;
        }
        wait("readyok")?;
        send(&format!("position fen {fen} moves {}", moves.join(" ")))?;
        send(&if analysis {
            "go nodes 500000 depth 20".into()
        } else {
            format!("go nodes {nodes} depth {depth}")
        })?;
        let until = Instant::now() + Duration::from_secs(30);
        let mut info = String::new();
        loop {
            if cancel.load(Ordering::Acquire) {
                return Err("Cancelled".into());
            }
            if Instant::now() > until {
                return Err("Engine search timed out; the position was kept".into());
            }
            match rx.recv_timeout(Duration::from_millis(30)) {
                Ok(line) => {
                    if let Some(best) = line.strip_prefix("bestmove ") {
                        return Ok((
                            best.split_whitespace().next().unwrap_or("").to_owned(),
                            info,
                        ));
                    }
                    if line.starts_with("info depth ") && line.contains(" score ") {
                        info = line;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(_) => {
                    return Err(
                        "Fairy-Stockfish exited during its search; the position was kept".into(),
                    );
                }
            }
        }
    })();
    drop(input);
    drop(process);
    drop(rx);
    let _ = reader.join();
    result
}
