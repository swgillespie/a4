// Copyright 2017-2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{collections::HashMap, fmt::Debug, io::Write, sync::Mutex, time::SystemTime};

use derive_more::From;
use serde::{Deserialize, Serialize};
use tracing::{
    field::{Field, Visit},
    span::Attributes,
    Event, Id, Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::tracing::constants;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchEvent {
    pub timestamp: SystemTime,
    pub kind: SearchEventKind,
}

#[derive(Debug, Serialize, Deserialize, From)]
pub enum SearchEventKind {
    Start(StartEvent),
    Instant(InstantEvent),
    End(EndEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartEvent {
    pub id: u64,
    pub kind: StartEventKind,
}

#[derive(Debug, Serialize, Deserialize, From)]
pub enum StartEventKind {
    Search(SearchStartEvent),
    SearchDepth(SearchDepthStartEvent),
    AlphaBeta(AlphaBetaStartEvent),
    AlphaBetaMove(AlphaBetaMoveStartEvent),
    AlphaBetaHashMove(AlphaBetaHashMoveStartEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchStartEvent {
    pub fen: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchDepthStartEvent {
    pub depth: u32,
    pub fen: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlphaBetaStartEvent {
    pub alpha: String,
    pub beta: String,
    pub depth: u32,
    pub fen: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlphaBetaMoveStartEvent {
    pub mov: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlphaBetaHashMoveStartEvent {
    pub mov: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstantEvent {
    pub kind: InstantEventKind,
}

#[derive(Debug, Serialize, Deserialize, From)]
pub enum InstantEventKind {
    SearchTermination(SearchTerminationEvent),
    SearchComplete(SearchCompleteEvent),
    SearchWithDepthComplete(SearchWithDepthCompleteEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchTerminationEvent {
    pub reason: SearchTerminationReason,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchCompleteEvent {
    pub best_move: String,
    pub best_value: String,
    pub nodes_evaluated: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchWithDepthCompleteEvent {
    pub best_move: String,
    pub best_value: String,
    pub nodes_evaluated: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SearchTerminationReason {
    Explicit,
    Time,
    Nodes,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EndEvent {
    pub id: u64,
    pub kind: EndEventKind,
}

#[derive(Debug, Serialize, Deserialize, From)]
pub enum EndEventKind {
    Search(SearchEndEvent),
    SearchDepth(SearchDepthEndEvent),
    AlphaBeta(AlphaBetaEndEvent),
    AlphaBetaMove(AlphaBetaMoveEndEvent),
    AlphaBetaHashMove(AlphaBetaHashMoveEndEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchEndEvent {}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchDepthEndEvent {}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlphaBetaEndEvent {}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlphaBetaMoveEndEvent {}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlphaBetaHashMoveEndEvent {}

/// The SearchGraphLayer is a Layer that specifically understands the instrumentation in a4's search routines and uses
/// them to reconstruct the search tree after a search is performed. It does not do any particular deep analysis of the
/// search; rather, it dumps the record of the search to disk for future analysis.
pub struct SearchGraphLayer {
    writer: Box<Mutex<dyn Write + Send + Sync>>,
}

impl SearchGraphLayer {
    pub fn new<W: Write + 'static + Send + Sync>(dest: W) -> SearchGraphLayer {
        SearchGraphLayer {
            writer: Box::new(Mutex::new(dest)),
        }
    }

    fn record_event<T: Into<SearchEventKind>>(&self, kind: T) {
        let event = SearchEvent {
            timestamp: SystemTime::now(),
            kind: kind.into(),
        };

        let mut writer = self.writer.lock().unwrap();
        serde_json::to_writer(&mut *writer, &event).expect("failed to write event");
        writeln!(&mut *writer, "").unwrap();
    }

    fn record_start_event<T: Into<StartEventKind>>(&self, id: &Id, kind: T) {
        let event = StartEvent {
            id: id.into_u64(),
            kind: kind.into(),
        };

        self.record_event(SearchEventKind::Start(event));
    }

    fn record_instant_event<T: Into<InstantEventKind>>(&self, kind: T) {
        let event = InstantEvent { kind: kind.into() };
        self.record_event(SearchEventKind::Instant(event));
    }

    fn record_end_event<T: Into<EndEventKind>>(&self, id: &Id, kind: T) {
        let event = EndEvent {
            id: id.into_u64(),
            kind: kind.into(),
        };

        self.record_event(SearchEventKind::End(event));
    }

    fn on_search_enter(&self, attrs: &Attributes<'_>, id: &Id) {
        let attrs = attrs.extract_fields();
        self.record_start_event(
            id,
            SearchStartEvent {
                fen: attrs.get("pos").expect("missing pos key").clone(),
            },
        );
    }

    fn on_search_exit(&self, id: &Id) {
        self.record_end_event(id, SearchEndEvent {});
        self.writer.lock().unwrap().flush().unwrap();
    }

    fn on_search_with_depth_enter(&self, attrs: &Attributes<'_>, id: &Id) {
        let attrs = attrs.extract_fields();
        self.record_start_event(
            id,
            SearchDepthStartEvent {
                depth: attrs
                    .get("depth")
                    .expect("missing depth key")
                    .parse()
                    .expect("depth wasn't an integer"),
                fen: attrs.get("pos").expect("missing pos key").clone(),
            },
        );
    }

    fn on_search_with_depth_exit(&self, id: &Id) {
        self.record_end_event(id, SearchDepthEndEvent {})
    }

    fn on_search_termination(&self, event: &Event<'_>) {
        let attrs = event.extract_fields();
        let termination_reason = match attrs.get("reason").expect("no reason key").as_ref() {
            "duration" => SearchTerminationReason::Time,
            "nodes" => SearchTerminationReason::Nodes,
            "explicit" => SearchTerminationReason::Explicit,
            r => panic!("unknown search termination reason: {}", r),
        };
        self.record_instant_event(SearchTerminationEvent {
            reason: termination_reason,
        });
    }

    fn on_search_complete(&self, event: &Event<'_>) {
        let attrs = event.extract_fields();
        self.record_instant_event(SearchCompleteEvent {
            best_move: attrs.get("best_move").unwrap().clone(),
            best_value: attrs.get("best_score").unwrap().clone(),
            nodes_evaluated: attrs.get("nodes").unwrap().parse().unwrap(),
        });
    }

    fn on_search_with_depth_complete(&self, event: &Event<'_>) {
        let attrs = event.extract_fields();
        self.record_instant_event(SearchWithDepthCompleteEvent {
            best_move: attrs.get("best_move").unwrap().clone(),
            best_value: attrs.get("best_score").unwrap().clone(),
            nodes_evaluated: attrs.get("nodes").unwrap().parse().unwrap(),
        });
    }

    fn on_alpha_beta_enter(&self, attrs: &Attributes<'_>, id: &Id) {
        let attrs = attrs.extract_fields();
        self.record_start_event(
            id,
            AlphaBetaStartEvent {
                alpha: attrs.get("alpha").unwrap().clone(),
                beta: attrs.get("beta").unwrap().clone(),
                fen: attrs.get("pos").unwrap().clone(),
                depth: attrs.get("depth").unwrap().parse().unwrap(),
            },
        )
    }

    fn on_alpha_beta_exit(&self, id: &Id) {
        self.record_end_event(id, AlphaBetaEndEvent {});
    }

    fn on_alpha_beta_move_enter(&self, attrs: &Attributes<'_>, id: &Id) {
        let attrs = attrs.extract_fields();
        self.record_start_event(
            id,
            AlphaBetaMoveStartEvent {
                mov: attrs.get("mov").unwrap().parse().unwrap(),
            },
        );
    }

    fn on_alpha_beta_move_exit(&self, id: &Id) {
        self.record_end_event(id, AlphaBetaMoveEndEvent {});
    }

    fn on_alpha_beta_hash_move_enter(&self, attrs: &Attributes<'_>, id: &Id) {
        let attrs = attrs.extract_fields();
        self.record_start_event(
            id,
            AlphaBetaHashMoveStartEvent {
                mov: attrs.get("hash_move").unwrap().clone(),
            },
        );
    }

    fn on_alpha_beta_hash_move_exit(&self, id: &Id) {
        self.record_end_event(id, AlphaBetaHashMoveEndEvent {})
    }
}

impl<S: Subscriber> Layer<S> for SearchGraphLayer
where
    S: for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).unwrap();
        match span.name() {
            constants::SEARCH => self.on_search_enter(attrs, id),
            constants::SEARCH_WITH_DEPTH => self.on_search_with_depth_enter(attrs, id),
            constants::ALPHA_BETA => self.on_alpha_beta_enter(attrs, id),
            constants::ALPHA_BETA_MOVE => self.on_alpha_beta_move_enter(attrs, id),
            constants::ALPHA_BETA_HASH_MOVE => self.on_alpha_beta_hash_move_enter(attrs, id),
            _ => {}
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).unwrap();
        match span.name() {
            constants::SEARCH => self.on_search_exit(&id),
            constants::SEARCH_WITH_DEPTH => self.on_search_with_depth_exit(&id),
            constants::ALPHA_BETA => self.on_alpha_beta_exit(&id),
            constants::ALPHA_BETA_MOVE => self.on_alpha_beta_move_exit(&id),
            constants::ALPHA_BETA_HASH_MOVE => self.on_alpha_beta_hash_move_exit(&id),
            _ => {}
        }
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let attrs = event.extract_fields();
        // Not all events have `event` keys (e.g. mundane logs from other modules).
        // Ignore the ones we don't care about.
        if let Some(event_str) = attrs.get("event") {
            match event_str.as_ref() {
                constants::SEARCH_TERMINATION => self.on_search_termination(event),
                constants::SEARCH_COMPLETE => self.on_search_complete(event),
                constants::SEARCH_WITH_DEPTH_COMPLETE => self.on_search_with_depth_complete(event),
                _ => {}
            }
        }
    }
}

trait HasExtractableFields {
    fn extract_fields(&self) -> HashMap<String, String>;
}

impl HasExtractableFields for Attributes<'_> {
    fn extract_fields(&self) -> HashMap<String, String> {
        let mut extractor = HashMapExtractor(HashMap::new());
        self.record(&mut extractor);
        extractor.0
    }
}

impl HasExtractableFields for Event<'_> {
    fn extract_fields(&self) -> HashMap<String, String> {
        let mut extractor = HashMapExtractor(HashMap::new());
        self.record(&mut extractor);
        extractor.0
    }
}

struct HashMapExtractor(HashMap<String, String>);
impl Visit for HashMapExtractor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0
            .insert(field.name().to_owned(), format!("{:?}", value));
    }
}
