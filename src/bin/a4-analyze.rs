// Copyright 2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code, unused_variables)] // Active development.

use std::{
    fs::File,
    io::{stdin, stdout, BufRead, BufReader, Write},
    path::PathBuf,
    time::SystemTime,
};

use a4::{
    position::Position,
    tracing::search::{
        EndEvent, EndEventKind, InstantEvent, InstantEventKind, SearchEvent, SearchEventKind,
        StartEvent, StartEventKind,
    },
};
use structopt::StructOpt;
use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};

/// Analyzer for search logs, as produced by a4-search.
#[derive(Debug, StructOpt)]
struct Options {
    /// A search log to analyze, as output by a4-search.
    #[structopt(name = "SEARCH_LOG")]
    search_log: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(LevelFilter::INFO)
        .with_env_filter(EnvFilter::from_env("A4_LOG"))
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let args = Options::from_args();
    let file = File::open(&args.search_log)?;
    let reader = BufReader::new(file);
    let mut events = vec![];
    for line in reader.lines() {
        let line = line?;
        let event: SearchEvent = serde_json::from_str(&line)?;
        events.push(event);
    }

    let builder = ObjectModelBuilder::default();
    let search = builder.from_events(events);
    repl(&search)
}

fn repl(search: &Search) -> anyhow::Result<()> {
    let mut stdin = BufReader::new(stdin());
    let mut stdout = stdout();
    let selected_search = Some(search);
    loop {
        let mut line = String::new();
        write!(&mut stdout, "a4> ")?;
        stdout.flush()?;
        stdin.read_line(&mut line)?;
        writeln!(&mut stdout, "{}", line.trim())?;
        let components: Vec<_> = line.trim().split_whitespace().collect();
        let (&command, arguments) = components.split_first().unwrap_or((&"", &[]));
        match (command, arguments) {
            ("info", []) => {
                if let Some(search) = selected_search {
                    let pos = Position::from_fen(search.fen.clone())?;
                    writeln!(&mut stdout, "== Search Position ==============")?;
                    writeln!(&mut stdout, "{}", pos)?;
                    writeln!(&mut stdout, "== Search Results ===============")?;
                    writeln!(&mut stdout, "{:<20} {}", "Best Move:", search.best_move)?;
                    writeln!(&mut stdout, "{:<20} {}", "Best Score:", search.best_score)?;
                    writeln!(
                        &mut stdout,
                        "{:<20} {}",
                        "Nodes Evaluated:", search.nodes_evaluated
                    )?;
                    writeln!(&mut stdout, "== Subsearches ===============")?;
                    writeln!(
                        &mut stdout,
                        "Search continued to depth {}",
                        search.subsearches.len()
                    )?;
                    for subsearch in &search.subsearches {
                        writeln!(
                            &mut stdout,
                            "==== Depth {} =================",
                            subsearch.depth
                        )?;
                        writeln!(&mut stdout, "{:<20} {}", "Best Move:", subsearch.best_move)?;
                        writeln!(
                            &mut stdout,
                            "{:<20} {}",
                            "Best Score:", subsearch.best_score
                        )?;
                        writeln!(
                            &mut stdout,
                            "{:<20} {}",
                            "Nodes Evaluated:", subsearch.nodes_evaluated
                        )?;
                    }
                }
            }
            (cmd, _) => {
                writeln!(&mut stdout, "unknown command {}", cmd)?;
            }
        }
    }
}

pub struct Search {
    fen: String,
    best_move: String,
    best_score: String,
    subsearches: Vec<SearchWithDepth>,
    nodes_evaluated: u64,
}

pub struct SearchWithDepth {
    id: u64,
    fen: String,
    depth: u32,
    best_move: String,
    best_score: String,
    nodes_evaluated: u64,
    searches: Vec<AlphaBeta>,
}

pub struct AlphaBeta {
    fen: String,
    alpha: String,
    beta: String,
    depth: u32,
    subsearches: Vec<AlphaBetaSubsearch>,
}

pub struct AlphaBetaSubsearch {
    search: AlphaBeta,
    mov: String,
}

#[derive(Default)]
struct SearchBuilder {
    id: u64,
    fen: String,
    depth: u32,
    best_move: Option<String>,
    best_score: Option<String>,
    nodes_evaluated: u64,
    searches: Vec<AlphaBeta>,
}

impl From<SearchBuilder> for SearchWithDepth {
    fn from(builder: SearchBuilder) -> SearchWithDepth {
        let SearchBuilder {
            id,
            fen,
            depth,
            best_move,
            best_score,
            nodes_evaluated,
            searches,
        } = builder;
        SearchWithDepth {
            id,
            fen,
            depth,
            best_move: best_move.unwrap(),
            best_score: best_score.unwrap(),
            nodes_evaluated,
            searches,
        }
    }
}

#[derive(Default)]
pub struct ObjectModelBuilder {
    search_fen: Option<String>,
    search_best_move: Option<String>,
    search_best_score: Option<String>,
    nodes_evaluated: u64,
    finished_searches: Vec<SearchWithDepth>,
    current_search: Option<SearchBuilder>,
}

impl From<ObjectModelBuilder> for Search {
    fn from(builder: ObjectModelBuilder) -> Search {
        Search {
            fen: builder.search_fen.expect("search had no fen?"),
            nodes_evaluated: builder.nodes_evaluated,
            best_move: builder.search_best_move.expect("search had no best move?"),
            best_score: builder
                .search_best_score
                .expect("search had no best score?"),
            subsearches: builder.finished_searches,
        }
    }
}

impl ObjectModelBuilder {
    fn from_events(mut self, events: Vec<SearchEvent>) -> Search {
        for event in events {
            let timestamp = event.timestamp;
            match event.kind {
                SearchEventKind::Start(start) => self.start_event(timestamp, start),
                SearchEventKind::Instant(instant) => self.instant_event(timestamp, instant),
                SearchEventKind::End(end) => self.end_event(timestamp, end),
            }
        }

        self.into()
    }

    fn start_event(&mut self, time: SystemTime, event: StartEvent) {
        let id = event.id;
        match event.kind {
            StartEventKind::Search(search) => {
                self.search_fen = Some(search.fen);
            }
            StartEventKind::SearchDepth(search_depth) => {
                assert!(
                    self.current_search.is_none(),
                    "recursive search depths not possible"
                );

                self.current_search = Some(SearchBuilder {
                    id,
                    fen: search_depth.fen,
                    depth: search_depth.depth,
                    ..Default::default()
                });
            }
            StartEventKind::AlphaBeta(ab) => {
                let searches = &mut self.current_search().searches;
                searches.push(AlphaBeta {
                    fen: ab.fen,
                    depth: ab.depth,
                    alpha: ab.alpha,
                    beta: ab.beta,
                    subsearches: vec![],
                })
            }
        }
    }

    fn instant_event(&mut self, time: SystemTime, event: InstantEvent) {
        match event.kind {
            InstantEventKind::SearchComplete(search_complete) => {
                self.search_best_move = Some(search_complete.best_move);
                self.search_best_score = Some(search_complete.best_value);
                self.nodes_evaluated = search_complete.nodes_evaluated;
            }
            InstantEventKind::SearchTermination(search_termination) => {}
            InstantEventKind::SearchWithDepthComplete(search_with_depth_complete) => {
                let search = self.current_search.as_mut().unwrap();
                search.best_move = Some(search_with_depth_complete.best_move);
                search.best_score = Some(search_with_depth_complete.best_value);
                search.nodes_evaluated = search_with_depth_complete.nodes_evaluated;
            }
        }
    }

    fn end_event(&mut self, time: SystemTime, event: EndEvent) {
        let id = event.id;
        match event.kind {
            EndEventKind::SearchDepth(search_depth) => {
                assert!(
                    self.current_search.is_some(),
                    "should be terminating a search?"
                );

                let finished_search: SearchWithDepth = self.current_search.take().unwrap().into();
                self.finished_searches.push(finished_search);
            }
            EndEventKind::Search(_) => {}
            EndEventKind::AlphaBeta(_) => {
                self.current_search().searches.pop();
            }
        }
    }

    fn current_search(&mut self) -> &mut SearchBuilder {
        self.current_search.as_mut().unwrap()
    }

    fn current_alpha_beta(&mut self) -> &mut AlphaBeta {
        self.current_search().searches.last_mut().unwrap()
    }
}
