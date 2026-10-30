use crate::{engine, protocol::*, rating};
use serde::{Deserialize, Serialize};
use shakmaty::{
    CastlingMode, Color, EnPassantMode, Position,
    fen::Fen,
    san::{San, SanPlus},
    uci::UciMove,
    variant::{Variant, VariantPosition},
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub const VARIANTS: [&str; 9] = [
    "chess",
    "chess960",
    "crazyhouse",
    "kingofthehill",
    "3check",
    "antichess",
    "atomic",
    "horde",
    "racingkings",
];
const REFERENCES: [f64; 8] = [600.0, 850.0, 1100.0, 1350.0, 1600.0, 1850.0, 2100.0, 2350.0];

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedGame {
    pub id: String,
    pub date: String,
    pub options: GameOptions,
    pub initial: String,
    pub moves: Vec<String>,
    pub white_ms: f64,
    pub black_ms: f64,
    pub result: String,
    pub reason: String,
    pub assisted: bool,
    #[serde(default)]
    pub review_only: bool,
}
#[derive(Default, Serialize, Deserialize)]
pub struct Store {
    pub schema: u32,
    pub active: Option<SavedGame>,
    pub games: Vec<SavedGame>,
    pub preferences: HashMap<String, String>,
    pub players: Vec<String>,
}
pub struct Service {
    pub store: Store,
    pub directory: PathBuf,
    pub engine: PathBuf,
    pub revision: u64,
    last_clock: Instant,
    pub analysis: String,
}
fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
fn mode(options: &GameOptions) -> CastlingMode {
    if options.variant == "chess960" {
        CastlingMode::Chess960
    } else {
        CastlingMode::Standard
    }
}
fn variant(options: &GameOptions) -> Result<Variant, String> {
    Variant::from_ascii(options.variant.as_bytes()).map_err(|e| e.to_string())
}
pub fn pool(options: &GameOptions) -> String {
    let seconds = options.minutes * 60 + options.increment * 40;
    let speed = if options.minutes == 0 {
        "Untimed"
    } else if seconds < 180 {
        "Bullet"
    } else if seconds < 480 {
        "Blitz"
    } else if seconds < 1500 {
        "Rapid"
    } else {
        "Classical"
    };
    format!("{} / {speed}", options.variant)
}
fn color_name(c: Color) -> &'static str {
    if c == Color::White { "white" } else { "black" }
}
fn fen(pos: &VariantPosition) -> String {
    Fen::from_position(pos, EnPassantMode::Legal).to_string()
}
pub fn initial_position(options: &GameOptions) -> Result<VariantPosition, String> {
    if !VARIANTS.contains(&options.variant.as_str()) {
        return Err("Unknown variant".into());
    }
    if !options.fen.trim().is_empty() {
        let setup = options
            .fen
            .parse::<Fen>()
            .map_err(|e| e.to_string())?
            .into_setup();
        return VariantPosition::from_setup(variant(options)?, setup, mode(options))
            .map_err(|e| format!("Invalid starting position: {e}"));
    }
    if options.variant == "chess960" {
        if options.chess960 > 959 {
            return Err("Chess960 position must be between 0 and 959".into());
        }
        let mut n = options.chess960 as usize;
        let mut pieces = [' '; 8];
        pieces[(n % 4) * 2 + 1] = 'B';
        n /= 4;
        pieces[(n % 4) * 2] = 'B';
        n /= 4;
        let empty: Vec<_> = (0..8).filter(|&i| pieces[i] == ' ').collect();
        pieces[empty[n % 6]] = 'Q';
        n /= 6;
        let empty: Vec<_> = (0..8).filter(|&i| pieces[i] == ' ').collect();
        let combinations = [
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (1, 2),
            (1, 3),
            (1, 4),
            (2, 3),
            (2, 4),
            (3, 4),
        ];
        let (a, b) = combinations[n];
        pieces[empty[a]] = 'N';
        pieces[empty[b]] = 'N';
        let empty: Vec<_> = (0..8).filter(|&i| pieces[i] == ' ').collect();
        pieces[empty[0]] = 'R';
        pieces[empty[1]] = 'K';
        pieces[empty[2]] = 'R';
        let row: String = pieces.iter().collect();
        let rights = format!(
            "{}{}{}{}",
            (b'A' + empty[0] as u8) as char,
            (b'A' + empty[2] as u8) as char,
            (b'a' + empty[0] as u8) as char,
            (b'a' + empty[2] as u8) as char
        );
        let setup = format!(
            "{}/pppppppp/8/8/8/8/PPPPPPPP/{row} w {rights} - 0 1",
            row.to_lowercase()
        )
        .parse::<Fen>()
        .map_err(|e| e.to_string())?
        .into_setup();
        return VariantPosition::from_setup(Variant::Chess, setup, CastlingMode::Chess960)
            .map_err(|e| e.to_string());
    }
    Ok(VariantPosition::new(variant(options)?))
}
pub fn positions(
    game: &SavedGame,
) -> Result<(VariantPosition, Vec<MoveRecord>, Vec<String>), String> {
    let mut options = game.options.clone();
    options.fen = game.initial.clone();
    let mut pos = initial_position(&options)?;
    let mut records = Vec::new();
    let mut repetitions = vec![position_key(&pos)];
    for text in &game.moves {
        let m = text
            .parse::<UciMove>()
            .map_err(|e| e.to_string())?
            .to_move(&pos)
            .map_err(|_| "Saved game contains an illegal move")?;
        let san = SanPlus::from_move_and_play_unchecked(&mut pos, m).to_string();
        records.push(MoveRecord {
            uci: text.clone(),
            san,
            fen: fen(&pos),
        });
        repetitions.push(position_key(&pos));
    }
    Ok((pos, records, repetitions))
}
fn position_key(pos: &VariantPosition) -> String {
    fen(pos)
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
}
impl Service {
    pub fn open(directory: &Path, engine: &Path) -> Result<Self, String> {
        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let path = directory.join("games.json");
        let store: Store = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Cannot read saved games; the file was preserved: {e}"))?
        } else {
            Store {
                schema: 1,
                players: vec!["Player".into()],
                ..Default::default()
            }
        };
        if store.schema != 1 {
            return Err("Unsupported saved-game version; the file was preserved".into());
        }
        if let Some(game) = &store.active {
            positions(game)?;
        }
        Ok(Self {
            store,
            directory: directory.to_path_buf(),
            engine: engine.to_path_buf(),
            revision: 1,
            last_clock: Instant::now(),
            analysis: String::new(),
        })
    }
    pub fn save(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.store).map_err(|e| e.to_string())?;
        let temp = self.directory.join("games.json.tmp");
        use std::io::Write;
        let mut f = fs::File::create(&temp).map_err(|e| e.to_string())?;
        f.write_all(&bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        if self.directory.join("games.json").exists() {
            fs::copy(
                self.directory.join("games.json"),
                self.directory.join("games.json.bak"),
            )
            .map_err(|e| e.to_string())?;
        }
        fs::rename(temp, self.directory.join("games.json")).map_err(|e| e.to_string())
    }
    fn check_revision(&self, revision: u64) -> Result<(), String> {
        if revision != self.revision {
            Err("The position changed; try the action again".into())
        } else {
            Ok(())
        }
    }
    fn active(&self) -> Result<&SavedGame, String> {
        self.store
            .active
            .as_ref()
            .ok_or("Choose New game to play".into())
    }
    pub fn tick(&mut self) -> Result<(), String> {
        let elapsed = self.last_clock.elapsed().as_secs_f64() * 1000.0;
        self.last_clock = Instant::now();
        let Some(game) = self.store.active.as_mut() else {
            return Ok(());
        };
        if game.result != "*"
            || game.review_only
            || game.options.minutes == 0
            || game.moves.is_empty()
        {
            return Ok(());
        }
        let (pos, _, _) = positions(game)?;
        let remaining = if pos.turn() == Color::White {
            &mut game.white_ms
        } else {
            &mut game.black_ms
        };
        *remaining = (*remaining - elapsed).max(0.0);
        if *remaining == 0.0 {
            let winner = !pos.turn();
            let can_win = !pos.has_insufficient_material(winner);
            self.finish(
                if can_win {
                    if winner == Color::White { "1-0" } else { "0-1" }
                } else {
                    "1/2-1/2"
                },
                "Time expired",
            )?;
        }
        Ok(())
    }
    fn finish(&mut self, result: &str, reason: &str) -> Result<(), String> {
        let game = self.store.active.as_mut().ok_or("No active game")?;
        if game.result != "*" {
            return Ok(());
        }
        game.result = result.to_owned();
        game.reason = reason.to_owned();
        self.store.games.push(game.clone());
        self.revision += 1;
        self.save()
    }
    pub fn new_game(&mut self, mut options: GameOptions) -> Result<(), String> {
        if !(1..=8).contains(&options.level)
            || !(0..=180).contains(&options.minutes)
            || !(0..=180).contains(&options.increment)
        {
            return Err("Invalid difficulty or clock settings".into());
        }
        if options.side == "random" {
            options.side = if stamp() % 2 == 0 { "white" } else { "black" }.into();
        }
        if options.side != "white" && options.side != "black" {
            return Err("Choose White, Black or Random".into());
        }
        options.player = options.player.trim().chars().take(60).collect();
        if options.player.is_empty() {
            options.player = "Player".into();
        }
        if options.rated && !options.fen.trim().is_empty() {
            return Err("Custom starting positions are practice games".into());
        }
        let pos = initial_position(&options)?;
        if pos.outcome().is_known() {
            return Err(
                "The starting position is already finished. Choose an earlier position.".into(),
            );
        }
        if self
            .store
            .active
            .as_ref()
            .is_some_and(|g| g.result == "*" && !g.review_only && !g.moves.is_empty())
        {
            return Err("Finish or resign the current game before starting another".into());
        }
        if !self.store.players.contains(&options.player) {
            self.store.players.push(options.player.clone());
        }
        let remaining = options.minutes as f64 * 60000.0;
        self.store.active = Some(SavedGame {
            id: stamp().to_string(),
            date: stamp().to_string(),
            initial: fen(&pos),
            options,
            moves: Vec::new(),
            white_ms: remaining,
            black_ms: remaining,
            result: "*".into(),
            reason: String::new(),
            assisted: false,
            review_only: false,
        });
        self.revision += 1;
        self.last_clock = Instant::now();
        self.analysis.clear();
        self.save()
    }
    pub fn play(&mut self, text: &str, human: bool, revision: u64) -> Result<(), String> {
        self.check_revision(revision)?;
        self.tick()?;
        let game = self.active()?;
        if game.result != "*" || game.review_only {
            return Err("This game is for review".into());
        }
        let (mut pos, _, _) = positions(game)?;
        if (color_name(pos.turn()) == game.options.side) != human {
            return Err("It is the other player's turn".into());
        }
        let m = text
            .parse::<UciMove>()
            .ok()
            .and_then(|m| m.to_move(&pos).ok())
            .or_else(|| text.parse::<San>().ok().and_then(|m| m.to_move(&pos).ok()))
            .ok_or("That move is not legal")?;
        let uci = m.to_uci(mode(&game.options)).to_string();
        let side = pos.turn();
        pos.play_unchecked(m);
        let game = self.store.active.as_mut().unwrap();
        game.moves.push(uci);
        if game.options.minutes > 0 {
            if side == Color::White {
                game.white_ms += game.options.increment as f64 * 1000.0;
            } else {
                game.black_ms += game.options.increment as f64 * 1000.0;
            }
        }
        self.revision += 1;
        self.last_clock = Instant::now();
        self.analysis.clear();
        let (_, _, reps) = positions(self.active()?)?;
        if pos.outcome().is_known() {
            self.finish(
                &pos.outcome().to_string(),
                if pos.is_checkmate() {
                    "Checkmate"
                } else if pos.is_stalemate() {
                    "Stalemate"
                } else if pos.is_variant_end() {
                    "Variant victory"
                } else {
                    "Insufficient material"
                },
            )?;
        } else if reps.iter().filter(|&k| k == reps.last().unwrap()).count() >= 5 {
            self.finish("1/2-1/2", "Fivefold repetition")?;
        } else if pos.halfmoves() >= 150 {
            self.finish("1/2-1/2", "75-move rule")?;
        }
        self.save()
    }
    pub fn computer(&mut self, revision: u64, cancel: &AtomicBool) -> Result<(), String> {
        self.check_revision(revision)?;
        self.tick()?;
        let game = self.active()?;
        let (pos, _, _) = positions(game)?;
        if game.result != "*" || game.review_only || color_name(pos.turn()) == game.options.side {
            return Err("The computer is not on move".into());
        }
        let (best, _) = engine::search(
            &self.engine,
            variant(&game.options)?.uci(),
            game.options.variant == "chess960",
            &game.initial,
            &game.moves,
            game.options.level,
            cancel,
            false,
        )?;
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Err("Cancelled".into());
        }
        self.play(&best, false, revision)
    }
    pub fn analyze(&mut self, index: usize, cancel: &AtomicBool) -> Result<(), String> {
        let mut game = self.active()?.clone();
        if game.result == "*" && !game.review_only {
            return Err("Analysis is available after the game".into());
        }
        game.moves.truncate(index.min(game.moves.len()));
        let (best, info) = engine::search(
            &self.engine,
            variant(&game.options)?.uci(),
            game.options.variant == "chess960",
            &game.initial,
            &game.moves,
            8,
            cancel,
            true,
        )?;
        self.analysis = format!("{info}\nbestmove {best}");
        Ok(())
    }
    pub fn handle(&mut self, request: &Request, cancel: &AtomicBool) -> Result<Snapshot, String> {
        match request.action.as_str() {
            "snapshot" => self.tick()?,
            "checkpoint" => {
                self.tick()?;
                self.save()?;
            }
            "import" => {
                self.check_revision(request.revision)?;
                if self
                    .store
                    .active
                    .as_ref()
                    .is_some_and(|g| g.result == "*" && !g.review_only && !g.moves.is_empty())
                {
                    return Err("Finish the current game before opening a PGN".into());
                }
                #[derive(Deserialize)]
                struct Import {
                    options: GameOptions,
                    moves: Vec<String>,
                    result: String,
                }
                let mut imported: Import =
                    serde_json::from_str(&request.text).map_err(|e| e.to_string())?;
                if imported.moves.len() > 20000 {
                    return Err("Game is too long".into());
                }
                imported.options.rated = false;
                imported.options.level = 1;
                imported.options.minutes = 0;
                if !["*", "1-0", "0-1", "1/2-1/2"].contains(&imported.result.as_str()) {
                    return Err("Invalid PGN result".into());
                }
                let initial = fen(&initial_position(&imported.options)?);
                let game = SavedGame {
                    id: stamp().to_string(),
                    date: stamp().to_string(),
                    options: imported.options,
                    initial,
                    moves: imported.moves,
                    white_ms: 0.0,
                    black_ms: 0.0,
                    result: imported.result,
                    reason: "Imported PGN".into(),
                    assisted: true,
                    review_only: true,
                };
                positions(&game)?;
                self.store.active = Some(game);
                self.revision += 1;
                self.analysis.clear();
                self.save()?;
            }
            "new" => {
                self.check_revision(request.revision)?;
                self.new_game(request.options.clone().ok_or("Missing game settings")?)?;
            }
            "move" => self.play(&request.text, true, request.revision)?,
            "computer" => self.computer(request.revision, cancel)?,
            "undo" => {
                self.check_revision(request.revision)?;
                self.tick()?;
                let game = self.store.active.as_mut().ok_or("No game")?;
                if game.result != "*" || game.review_only {
                    return Err("Finished games cannot be changed".into());
                }
                if game.options.rated {
                    return Err("Takebacks are available in practice games".into());
                }
                if !game.moves.is_empty() {
                    game.moves.pop();
                }
                while !game.moves.is_empty()
                    && color_name(positions(game)?.0.turn()) != game.options.side
                {
                    game.moves.pop();
                }
                game.assisted = true;
                self.revision += 1;
                self.last_clock = Instant::now();
                self.save()?;
            }
            "resign" => {
                self.check_revision(request.revision)?;
                let side = self.active()?.options.side.clone();
                self.finish(if side == "white" { "0-1" } else { "1-0" }, "Resignation")?;
            }
            "claim" => {
                self.check_revision(request.revision)?;
                if !self.snapshot()?.can_claim {
                    return Err("No draw can be claimed in this position".into());
                }
                self.finish("1/2-1/2", "Draw claimed")?;
            }
            "practice" => {
                self.check_revision(request.revision)?;
                let game = self.store.active.as_mut().ok_or("No game")?;
                if game.result != "*" {
                    return Err("Game finished".into());
                }
                game.options.rated = false;
                game.assisted = true;
                self.revision += 1;
                self.save()?;
            }
            "history" => {
                self.check_revision(request.revision)?;
                if self
                    .store
                    .active
                    .as_ref()
                    .is_some_and(|g| g.result == "*" && !g.review_only && !g.moves.is_empty())
                {
                    return Err("Finish the current game before opening another".into());
                }
                self.store.active = Some(
                    self.store
                        .games
                        .iter()
                        .find(|g| g.id == request.text)
                        .ok_or("Saved game not found")?
                        .clone(),
                );
                self.revision += 1;
                self.analysis.clear();
                self.last_clock = Instant::now();
            }
            "preference" => {
                if request.key.len() > 64 || request.value.len() > 256 {
                    return Err("Preference is too long".into());
                }
                self.store
                    .preferences
                    .insert(request.key.clone(), request.value.clone());
                self.revision += 1;
                self.save()?;
            }
            "analyze" => {
                self.check_revision(request.revision)?;
                self.analyze(request.index.max(0) as usize, cancel)?;
            }
            _ => return Err("Unknown command".into()),
        }
        self.snapshot()
    }
    pub fn snapshot(&self) -> Result<Snapshot, String> {
        let mut snapshot = Snapshot {
            revision: self.revision,
            status: "empty".into(),
            engine: "Fairy-Stockfish 14".into(),
            history: self
                .store
                .games
                .iter()
                .rev()
                .map(|g| GameSummary {
                    id: g.id.clone(),
                    date: g.date.clone(),
                    variant: g.options.variant.clone(),
                    level: g.options.level,
                    result: g.result.clone(),
                    rated: g.options.rated && !g.assisted,
                    player: g.options.player.clone(),
                    time_control: pool(&g.options)
                        .split(" / ")
                        .nth(1)
                        .unwrap_or("")
                        .to_string(),
                    plies: g.moves.len() as u32,
                })
                .collect(),
            preferences: self.store.preferences.clone(),
            players: self.store.players.clone(),
            analysis: self.analysis.clone(),
            ..Default::default()
        };
        if let Some(game) = &self.store.active {
            let (pos, moves, reps) = positions(game)?;
            snapshot.id = game.id.clone();
            snapshot.options = Some(game.options.clone());
            snapshot.fen = fen(&pos);
            snapshot.initial_fen = game.initial.clone();
            snapshot.turn = color_name(pos.turn()).into();
            snapshot.moves = moves;
            snapshot.status = if game.review_only {
                "review"
            } else if game.result == "*" {
                "playing"
            } else {
                "finished"
            }
            .into();
            snapshot.result = game.result.clone();
            snapshot.reason = game.reason.clone();
            snapshot.check = pos.is_check();
            snapshot.can_claim = game.result == "*"
                && (pos.halfmoves() >= 100
                    || reps.iter().filter(|&k| k == reps.last().unwrap()).count() >= 3);
            snapshot.white_ms = game.white_ms;
            snapshot.black_ms = game.black_ms;
            snapshot.legal = pos
                .legal_moves()
                .iter()
                .map(|m| {
                    let uci = m.to_uci(mode(&game.options));
                    match uci {
                        UciMove::Normal {
                            from,
                            to,
                            promotion,
                        } => LegalMove {
                            uci: uci.to_string(),
                            from: from.to_string(),
                            to: to.to_string(),
                            promotion: promotion.map(|p| p.char().to_string()).unwrap_or_default(),
                            role: String::new(),
                        },
                        UciMove::Put { role, to } => LegalMove {
                            uci: uci.to_string(),
                            from: String::new(),
                            to: to.to_string(),
                            promotion: String::new(),
                            role: role.char().to_string(),
                        },
                        UciMove::Null => LegalMove::default(),
                    }
                })
                .collect();
            snapshot.pgn = export(game, &snapshot.moves);
            snapshot.progress = Some(self.progress(&game.options));
            snapshot.rating_note = "Local progress score uses Glicko-2 against fixed app reference values. It is not a Lichess or FIDE rating, and reference levels are not calibrated human Elo.".into();
        }
        Ok(snapshot)
    }
    fn progress(&self, options: &GameOptions) -> Progress {
        let key = pool(options);
        let mut r = rating::initial();
        let mut levels: Vec<LevelResult> = (1..=8)
            .map(|level| LevelResult {
                level,
                ..Default::default()
            })
            .collect();
        let mut trend = Vec::new();
        for game in &self.store.games {
            if game.options.player != options.player
                || pool(&game.options) != key
                || game.result == "*"
                || game.review_only
            {
                continue;
            }
            let score = if game.result == "1/2-1/2" {
                0.5
            } else if (game.result == "1-0") == (game.options.side == "white") {
                1.0
            } else {
                0.0
            };
            let level = &mut levels[(game.options.level - 1) as usize];
            if score == 1.0 {
                level.wins += 1;
            } else if score == 0.0 {
                level.losses += 1;
            } else {
                level.draws += 1;
            }
            if game.options.rated && !game.assisted {
                r = rating::update(
                    &r,
                    &[(REFERENCES[(game.options.level - 1) as usize], 200.0, score)],
                );
                trend.push(r.value);
            }
        }
        Progress {
            pool: key,
            rating: Some(r),
            levels,
            trend,
            calibrated: false,
        }
    }
}
fn quote(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\r', '\n'], " ")
}
pub fn export(game: &SavedGame, moves: &[MoveRecord]) -> String {
    let player = quote(&game.options.player);
    let computer = format!("Fairy-Stockfish 14 / Level {}", game.options.level);
    let (white, black) = if game.options.side == "white" {
        (player, computer)
    } else {
        (computer, player)
    };
    let variant = match game.options.variant.as_str() {
        "chess960" => "Chess960",
        "kingofthehill" => "King of the Hill",
        "3check" => "Three-check",
        "racingkings" => "Racing Kings",
        "crazyhouse" => "Crazyhouse",
        "antichess" => "Antichess",
        "atomic" => "Atomic",
        "horde" => "Horde",
        _ => "Standard",
    };
    let mut text = format!(
        "[Event \"Lichess Local computer game\"]\n[Site \"Local\"]\n[White \"{white}\"]\n[Black \"{black}\"]\n[Result \"{}\"]\n[Variant \"{variant}\"]\n[SetUp \"1\"]\n[FEN \"{}\"]\n[TimeControl \"{}\"]\n\n",
        game.result,
        game.initial,
        if game.options.minutes == 0 {
            "-".into()
        } else {
            format!("{}+{}", game.options.minutes * 60, game.options.increment)
        }
    );
    let initial: Vec<_> = game.initial.split_whitespace().collect();
    let mut number = initial
        .last()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);
    let mut white_turn = initial.get(1) != Some(&"b");
    for (i, m) in moves.iter().enumerate() {
        if white_turn {
            text.push_str(&format!("{number}. "));
        } else if i == 0 {
            text.push_str(&format!("{number}... "));
        }
        text.push_str(&m.san);
        text.push(' ');
        if !white_turn {
            number += 1;
        }
        white_turn = !white_turn;
    }
    text.push_str(&game.result);
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options(v: &str) -> GameOptions {
        GameOptions {
            variant: v.into(),
            side: "white".into(),
            level: 1,
            player: "Test".into(),
            ..Default::default()
        }
    }
    #[test]
    fn all_variants_have_legal_starts() {
        for v in VARIANTS {
            let p = initial_position(&options(v)).unwrap();
            assert!(!p.legal_moves().is_empty(), "{v}");
        }
    }
    #[test]
    fn chess960_positions_are_unique_and_standard_is_518() {
        let mut seen = std::collections::HashSet::new();
        for n in 0..960 {
            let mut o = options("chess960");
            o.chess960 = n;
            seen.insert(fen(&initial_position(&o).unwrap()));
        }
        assert_eq!(seen.len(), 960);
        let mut o = options("chess960");
        o.chess960 = 518;
        assert!(fen(&initial_position(&o).unwrap()).starts_with("rnbqkbnr/"));
    }
    #[test]
    fn stale_moves_rejected_and_save_resumes() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Service::open(dir.path(), Path::new("missing")).unwrap();
        s.new_game(options("chess")).unwrap();
        assert!(s.play("e2e4", true, 1).is_err());
        s.play("e2e4", true, s.revision).unwrap();
        assert!(s.play("e7e5", true, s.revision).is_err());
        assert!(s.play("e7e4", false, s.revision).is_err());
        let other = Service::open(dir.path(), Path::new("missing")).unwrap();
        assert_eq!(other.snapshot().unwrap().moves[0].san, "e4");
    }
    #[test]
    fn checkmate_is_recorded_once_and_cannot_be_undone() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Service::open(dir.path(), Path::new("missing")).unwrap();
        s.new_game(options("chess")).unwrap();
        for (i, m) in ["f2f3", "e7e5", "g2g4", "d8h4"].iter().enumerate() {
            s.play(m, i % 2 == 0, s.revision).unwrap();
        }
        assert_eq!(s.snapshot().unwrap().result, "0-1");
        assert_eq!(s.store.games.len(), 1);
        assert!(
            s.handle(
                &Request {
                    action: "undo".into(),
                    revision: s.revision,
                    ..Default::default()
                },
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    #[test]
    fn rated_games_reject_takebacks_and_custom_positions() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Service::open(dir.path(), Path::new("missing")).unwrap();
        let mut o = options("chess");
        o.rated = true;
        s.new_game(o).unwrap();
        assert!(
            s.handle(
                &Request {
                    action: "undo".into(),
                    revision: s.revision,
                    ..Default::default()
                },
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    #[test]
    fn corrupt_saved_data_is_preserved() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("games.json"), b"bad").unwrap();
        assert!(Service::open(d.path(), Path::new("none")).is_err());
        assert_eq!(fs::read(d.path().join("games.json")).unwrap(), b"bad");
    }
    fn position_game(variant: &str, start: &str) -> (tempfile::TempDir, Service) {
        let d = tempfile::tempdir().unwrap();
        let mut s = Service::open(d.path(), Path::new("none")).unwrap();
        let mut o = options(variant);
        o.fen = start.into();
        s.new_game(o).unwrap();
        (d, s)
    }
    #[test]
    fn crazyhouse_drop_is_validated_and_pocket_is_consumed() {
        let (_d, mut s) = position_game("crazyhouse", "4k3/8/8/8/8/8/8/4K3[P] w - - 0 1");
        s.play("P@e4", true, s.revision).unwrap();
        let snap = s.snapshot().unwrap();
        assert!(snap.fen.contains("4P3"));
        assert!(!snap.fen.contains("[P]"));
    }
    #[test]
    fn atomic_explosion_ends_game() {
        let (_d, mut s) = position_game("atomic", "4k3/4p3/8/8/8/8/8/K3R3 w - - 0 1");
        s.play("e1e7", true, s.revision).unwrap();
        assert_eq!(s.snapshot().unwrap().result, "1-0");
    }
    #[test]
    fn hill_and_threecheck_victories() {
        for (v, f, m) in [
            ("kingofthehill", "7k/8/8/8/8/2K5/8/8 w - - 0 1", "c3d4"),
            ("3check", "4k3/8/8/8/8/8/R7/K7 w - - 1+3 0 1", "a2e2"),
        ] {
            let (_d, mut s) = position_game(v, f);
            s.play(m, true, s.revision).unwrap();
            assert_eq!(s.snapshot().unwrap().result, "1-0", "{v}");
        }
    }
    #[test]
    fn antichess_can_promote_to_king() {
        let (_d, mut s) = position_game("antichess", "8/P6p/8/8/8/8/8/7k w - - 0 1");
        assert!(
            s.snapshot()
                .unwrap()
                .legal
                .iter()
                .any(|m| m.promotion == "k")
        );
        s.play("a7a8k", true, s.revision).unwrap();
    }
    #[test]
    fn chess960_castle_with_stationary_king() {
        let (_d, mut s) = position_game("chess960", "4k3/8/8/8/8/8/8/6KR w H - 0 1");
        s.play("g1h1", true, s.revision).unwrap();
        assert!(s.snapshot().unwrap().fen.contains("5RK1"));
    }
    #[test]
    fn repetition_claim_and_time_expiry_are_settled() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Service::open(d.path(), Path::new("none")).unwrap();
        let mut o = options("chess");
        o.minutes = 1;
        s.new_game(o).unwrap();
        for i in 0..8 {
            let moves = ["g1f3", "g8f6", "f3g1", "f6g8"];
            s.play(moves[i % 4], i % 2 == 0, s.revision).unwrap();
        }
        assert!(s.snapshot().unwrap().can_claim);
        s.handle(
            &Request {
                action: "claim".into(),
                revision: s.revision,
                ..Default::default()
            },
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(s.snapshot().unwrap().result, "1/2-1/2");
        let mut o = options("chess");
        o.minutes = 1;
        s.new_game(o).unwrap();
        s.play("e4", true, s.revision).unwrap();
        s.play("e5", false, s.revision).unwrap();
        s.last_clock = Instant::now() - std::time::Duration::from_secs(61);
        s.tick().unwrap();
        assert_eq!(s.snapshot().unwrap().result, "0-1");
        assert_eq!(s.store.games.len(), 2);
    }
}
