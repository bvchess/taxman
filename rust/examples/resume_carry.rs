//! Scratch experiment: a search stopped every `--every` nodes and resumed from its checkpoint, with its nogood
//! tables starting empty on each resume or with the nogoods the stopped run ended with.
//!
//! ```text
//! cargo run --release --example resume_carry -- [--threads T] [--every NODES] [--known FILE] N...
//! ```

use std::path::PathBuf;
use std::time::{Duration, Instant};
use taxman::bits::Bits;
use taxman::engine::EngineConfig;
use taxman::frames::build_frames;
use taxman::game::Optimal;
use taxman::hives::Apiary;
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::search::{prepare, search_with, SearchTactics};

fn known_games(known: Option<&PathBuf>) -> Optimal {
    let json = Json::from_file(Optimal::default_path()).expect("optimal.json");
    let mut games: Vec<Json> = json.as_arr().expect("array").to_vec();
    if let Some(path) = known {
        let text = std::fs::read_to_string(path).expect("known file");
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let game = Json::parse(line).expect("json line");
            if game.get("completed").and_then(Json::as_bool) == Some(true) {
                let n = game.get("n").and_then(Json::as_u32);
                games.retain(|g| g.get("n").and_then(Json::as_u32) != n);
                games.push(game);
            }
        }
    }
    Optimal::from_json(&Json::Arr(games)).expect("games")
}

fn main() {
    let mut threads = 1;
    let mut every = 1_000_000u64;
    let mut known = None;
    let mut games = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--threads" => threads = args.next().unwrap().parse().unwrap(),
            "--every" => every = args.next().unwrap().parse().unwrap(),
            "--known" => known = Some(PathBuf::from(args.next().unwrap())),
            n => games.push(n.parse::<u32>().unwrap()),
        }
    }
    let optimal = known_games(known.as_ref());
    for n in games {
        for stored in [false, true] {
            run::<32>(n, threads, every, stored, &optimal);
        }
    }
}

fn run<const W: usize>(n: u32, threads: usize, every: u64, stored: bool, optimal: &Optimal) {
    let tables = Tables::new(n);
    let frames = build_frames::<W>(&tables);
    let columns: Vec<Vec<u32>> = frames.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect();
    let tactics = SearchTactics { revise_freedoms: false, ..SearchTactics::default() };
    let p = prepare::<W>(&tables, &columns, &frames.candidates, tactics, threads).expect("prepare");
    let in_the_bag: u64 = Apiary::<W>::new(&tables, Bits::EMPTY).solution().iter().map(|&x| u64::from(x)).sum();
    let cap = optimal.score(n - 1).map(|s| u64::from(n) + s - in_the_bag);
    let seed: Option<Bits<W>> = optimal.moves(n - 1).map(|m| m.iter().copied().collect());
    let dir = std::env::temp_dir().join(format!("taxman-resume-carry-{}-{n}-{stored}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = EngineConfig {
        threads,
        checkpoint: Some((dir.join("ckpt"), Duration::from_secs(3600))),
        node_limit: Some(every),
        spill_dir: Some(dir.join("spill")),
        ..EngineConfig::default()
    };
    let t0 = Instant::now();
    let mut carried: Vec<Bits<W>> = Vec::new();
    let mut pieces = 0;
    loop {
        let r = search_with(&p, &cfg, seed.as_ref(), cap, pieces > 0, &carried, None).expect("search");
        pieces += 1;
        if r.completed {
            println!(
                "n={n} T={threads} every {every} stored table {stored:5}: {pieces} pieces, {} nodes, {:.2} s, best {}, \
                 last table {}",
                r.stats.nodes,
                t0.elapsed().as_secs_f64(),
                r.sum,
                r.nogoods.len()
            );
            break;
        }
        if stored {
            carried = r.nogoods;
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
