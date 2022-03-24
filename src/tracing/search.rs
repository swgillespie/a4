// Copyright 2017-2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{collections::HashMap, fmt::Debug, io::Write, sync::Mutex, time::SystemTime};

use derive_more::From;
use serde::Serialize;
use tracing::{
    field::{Field, Visit},
    span::Attributes,
    Event, Id, Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::tracing::constants;

#[derive(Debug, Serialize)]
pub struct SearchEvent {
    timestamp: SystemTime,
    kind: SearchEventKind,
}

#[derive(Debug, Serialize, From)]
pub enum SearchEventKind {
    Start(StartEvent),
    Instant(InstantEvent),
    End(EndEvent),
}

#[derive(Debug, Serialize)]
pub struct StartEvent {
    id: u64,
    kind: StartEventKind,
}

#[derive(Debug, Serialize, From)]
pub enum StartEventKind {
    Search(SearchStartEvent),
    SearchDepth(SearchDepthStartEvent),
}

#[derive(Debug, Serialize)]
pub struct SearchStartEvent {
    fen: String,
}

#[derive(Debug, Serialize)]
pub struct SearchDepthStartEvent {
    depth: u32,
    fen: String,
}

#[derive(Debug, Serialize)]
pub struct InstantEvent {
    kind: InstantEventKind,
}

#[derive(Debug, Serialize, From)]
pub enum InstantEventKind {
    SearchTermination(SearchTerminationEvent),
}

#[derive(Debug, Serialize)]
pub struct SearchTerminationEvent {}

#[derive(Debug, Serialize)]
pub struct EndEvent {
    id: u64,
    kind: EndEventKind,
}

#[derive(Debug, Serialize, From)]
pub enum EndEventKind {
    Search(SearchEndEvent),
    SearchDepth(SearchDepthEndEvent),
}

#[derive(Debug, Serialize)]
pub struct SearchEndEvent {}

#[derive(Debug, Serialize)]
pub struct SearchDepthEndEvent {}

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
        let attrs = extract_fields(attrs);
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
        let attrs = extract_fields(attrs);
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

    fn on_search_termination(&self) {
        self.record_instant_event(SearchTerminationEvent {})
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
            _ => {}
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).unwrap();
        match span.name() {
            constants::SEARCH => self.on_search_exit(&id),
            constants::SEARCH_WITH_DEPTH => self.on_search_with_depth_exit(&id),
            _ => {}
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        todo!()
    }
}

fn extract_fields(attrs: &Attributes<'_>) -> HashMap<String, String> {
    struct HashMapExtractor(HashMap<String, String>);

    impl Visit for HashMapExtractor {
        fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
            self.0
                .insert(field.name().to_owned(), format!("{:?}", value));
        }
    }

    let mut extractor = HashMapExtractor(HashMap::new());
    attrs.record(&mut extractor);
    extractor.0
}
