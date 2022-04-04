// Copyright 2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code, unused_variables)] // Active development.

use std::{
    cell::RefCell,
    fs::File,
    io::{stdin, stdout, BufRead, BufReader, Write},
    path::PathBuf,
    rc::Rc,
    time::SystemTime,
};

use a4::{
    eval,
    position::Position,
    tracing::search::{
        EndEvent, EndEventKind, InstantEvent, InstantEventKind, SearchEvent, SearchEventKind,
        SearchTerminationReason, StartEvent, StartEventKind,
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
    let mut selected_subsearch = None;
    loop {
        let mut line = String::new();
        write!(&mut stdout, "a4> ")?;
        stdout.flush()?;
        stdin.read_line(&mut line)?;
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
                        match subsearch.termination {
                            Termination::Complete {
                                ref best_move,
                                ref best_score,
                                nodes_evaluated,
                            } => {
                                writeln!(&mut stdout, "{:<20} {}", "Best Move:", best_move)?;
                                writeln!(&mut stdout, "{:<20} {}", "Best Score:", best_score)?;
                                writeln!(
                                    &mut stdout,
                                    "{:<20} {}",
                                    "Nodes Evaluated:", nodes_evaluated
                                )?;
                            }
                            Termination::Premature { ref reason } => {
                                writeln!(&mut stdout, "Terminated Prematurely")?;
                                writeln!(&mut stdout, "Reason: {reason:?}")?;
                            }
                        }
                    }
                }
            }
            ("subsearch", ["list"]) => {
                if let Some(selected) = selected_search {
                    for (i, subsearches) in selected.subsearches.iter().enumerate() {
                        match subsearches.termination {
                            Termination::Complete {
                                ref best_move,
                                ref best_score,
                                nodes_evaluated,
                            } => {
                                writeln!(
                                    &mut stdout,
                                    "{:>3}. Depth {:<2} {:<5} ({})",
                                    i, subsearches.depth, best_move, best_score,
                                )?;
                            }
                            Termination::Premature { ref reason } => {
                                writeln!(
                                    &mut stdout,
                                    "{:>3}. Depth {:<2}, Terminated Early: {:?}",
                                    i, subsearches.depth, reason
                                )?;
                            }
                        }
                    }
                } else {
                    writeln!(&mut stdout, "no search selected")?;
                }
            }
            ("subsearch", ["select", idx]) => {
                if let Some(selected) = selected_search {
                    if let Some(subsearch) = selected.subsearches.get(idx.parse::<usize>()?) {
                        selected_subsearch = Some(subsearch);
                        writeln!(&mut stdout, "subsearch {} selected", idx)?;
                    } else {
                        writeln!(&mut stdout, "subsearch index out of bounds")?;
                    }
                } else {
                    writeln!(&mut stdout, "no search selected")?;
                }
            }
            ("alphabeta", ["list"]) => {
                if let Some(subsearch) = selected_subsearch {
                    writeln!(&mut stdout, "== {}", subsearch.ab.fen)?;
                    if let Some(ref ab) = subsearch.ab.hash_move_subsearch {
                        writeln!(
                            &mut stdout,
                            "Hash. {} [{}, {}]",
                            ab.mov, ab.search.alpha, ab.search.beta
                        )?;
                    }
                    for (i, ab) in subsearch.ab.subsearches.iter().enumerate() {
                        writeln!(
                            &mut stdout,
                            "{:>4}. {} [{}, {}]",
                            i, ab.mov, ab.search.alpha, ab.search.beta
                        )?;
                    }

                    writeln!(
                        &mut stdout,
                        "Searched {} positions",
                        subsearch.ab.subsearches.len()
                    )?;
                } else {
                    writeln!(&mut stdout, "no subsearch selected")?;
                }
            }
            ("eval", fen) => {
                if let Ok(pos) = Position::from_fen(fen.join(" ")) {
                    let score = eval::evaluate(&pos);
                    writeln!(&mut stdout, "{}", score)?;
                } else {
                    writeln!(&mut stdout, "invalid fen")?;
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
    termination: Termination,
    ab: AlphaBeta,
}

pub struct AlphaBeta {
    fen: String,
    alpha: String,
    beta: String,
    hash_move_subsearch: Option<Box<AlphaBetaSubsearch>>,
    subsearches: Vec<AlphaBetaSubsearch>,
}

pub struct AlphaBetaSubsearch {
    search: AlphaBeta,
    mov: String,
}

#[derive(Debug, Default)]
struct SearchBuilder {
    id: u64,
    fen: String,
    depth: u32,
    termination: Option<Termination>,
    ab: Option<Rc<RefCell<AlphaBetaBuilder>>>,
}

#[derive(Debug)]
enum Termination {
    Complete {
        best_move: String,
        best_score: String,
        nodes_evaluated: u64,
    },
    Premature {
        reason: SearchTerminationReason,
    },
}

impl From<SearchBuilder> for SearchWithDepth {
    fn from(builder: SearchBuilder) -> SearchWithDepth {
        let SearchBuilder {
            id,
            fen,
            depth,
            termination,
            ab,
        } = builder;
        let maybe_ab = ab.unwrap();
        let ab_ref = maybe_ab.borrow();
        assert!(
            termination.is_some(),
            "search builder for {} (depth {}) is incomplete",
            fen,
            depth
        );
        SearchWithDepth {
            id,
            fen,
            depth,
            termination: termination.unwrap(),
            ab: ab_ref.clone().into(),
        }
    }
}

#[derive(Default, Debug, Clone)]
struct AlphaBetaBuilder {
    alpha: Option<String>,
    beta: Option<String>,
    fen: Option<String>,
    hash_move: Option<(String, AlphaBetaBuilderRef)>,
    moves: Vec<(String, Rc<RefCell<AlphaBetaBuilder>>)>,
}

impl AlphaBetaBuilder {
    pub fn new() -> Self {
        Default::default()
    }
}

impl From<AlphaBetaBuilder> for AlphaBeta {
    fn from(builder: AlphaBetaBuilder) -> Self {
        let move_children = builder
            .moves
            .into_iter()
            .map(|(mov, builder)| {
                let ab_ref = builder.borrow();
                AlphaBetaSubsearch {
                    mov,
                    search: ab_ref.clone().into(),
                }
            })
            .collect();

        let hash_move = builder.hash_move.map(|(mov, builder)| {
            let ab_ref = builder.borrow();
            Box::new(AlphaBetaSubsearch {
                mov,
                search: ab_ref.clone().into(),
            })
        });
        AlphaBeta {
            fen: builder.fen.unwrap(),
            alpha: builder.alpha.unwrap(),
            beta: builder.beta.unwrap(),
            hash_move_subsearch: hash_move,
            subsearches: move_children,
        }
    }
}

type AlphaBetaBuilderRef = Rc<RefCell<AlphaBetaBuilder>>;

#[derive(Default)]
pub struct ObjectModelBuilder {
    search_fen: Option<String>,
    search_best_move: Option<String>,
    search_best_score: Option<String>,
    nodes_evaluated: u64,
    finished_searches: Vec<SearchWithDepth>,
    current_search: Option<SearchBuilder>,
    ab_stack: Vec<AlphaBetaBuilderRef>,
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
                assert!(self.ab_stack.is_empty(), "ab stack should be empty");

                self.current_search = Some(SearchBuilder {
                    id,
                    fen: search_depth.fen,
                    depth: search_depth.depth,
                    ..Default::default()
                });
                self.ab_stack
                    .push(Rc::new(RefCell::new(AlphaBetaBuilder::new())));
            }
            StartEventKind::AlphaBeta(ab) => {
                let current_ab = self.current_alpha_beta();
                let mut current_ab_mut = current_ab.borrow_mut();
                current_ab_mut.alpha = Some(ab.alpha);
                current_ab_mut.beta = Some(ab.beta);
                current_ab_mut.fen = Some(ab.fen);
            }
            StartEventKind::AlphaBetaMove(ab_move) => {
                let new_ab = {
                    let current_ab = self.current_alpha_beta();
                    let mut current_ab_mut = current_ab.borrow_mut();
                    let new_ab = Rc::new(RefCell::new(AlphaBetaBuilder::new()));
                    current_ab_mut.moves.push((ab_move.mov, new_ab.clone()));
                    new_ab
                };

                self.ab_stack.push(new_ab);
            }

            StartEventKind::AlphaBetaHashMove(ab_hash_move) => {
                let new_ab = {
                    let current_ab = self.current_alpha_beta();
                    let mut current_ab_mut = current_ab.borrow_mut();
                    let new_ab = Rc::new(RefCell::new(AlphaBetaBuilder::new()));
                    current_ab_mut.hash_move = Some((ab_hash_move.mov, new_ab.clone()));
                    new_ab
                };

                self.ab_stack.push(new_ab);
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
            InstantEventKind::SearchTermination(search_termination) => {
                let search = self.current_search.as_mut().unwrap();
                search.termination = Some(Termination::Premature {
                    reason: search_termination.reason,
                });
            }
            InstantEventKind::SearchWithDepthComplete(search_with_depth_complete) => {
                let search = self.current_search.as_mut().unwrap();
                search.termination = Some(Termination::Complete {
                    best_move: search_with_depth_complete.best_move,
                    best_score: search_with_depth_complete.best_value,
                    nodes_evaluated: search_with_depth_complete.nodes_evaluated,
                });
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

                self.current_search().ab = Some(self.current_alpha_beta().clone());
                let finished_search: SearchWithDepth = self.current_search.take().unwrap().into();
                self.finished_searches.push(finished_search);
                self.ab_stack.pop();
            }
            EndEventKind::Search(_) => {}
            EndEventKind::AlphaBeta(_) => {}
            EndEventKind::AlphaBetaMove(_) | EndEventKind::AlphaBetaHashMove(_) => {
                self.ab_stack.pop();
            }
        }
    }

    fn current_search(&mut self) -> &mut SearchBuilder {
        self.current_search.as_mut().unwrap()
    }

    fn current_alpha_beta(&self) -> AlphaBetaBuilderRef {
        self.ab_stack.last().cloned().unwrap()
    }
}
