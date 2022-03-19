// Copyright 2017-2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use tracing::{span::Attributes, Event, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::tracing::constants;

/// The SearchGraphLayer is a Layer that specifically understands the instrumentation in a4's search routines and uses
/// them to reconstruct the search tree after a search is performed. It does not do any particular deep analysis of the
/// search; rather, it dumps the record of the search to disk for future analysis.
pub struct SearchGraphLayer {}

impl SearchGraphLayer {
    pub fn new() -> SearchGraphLayer {
        SearchGraphLayer {}
    }

    fn on_search_enter(&self, _attrs: &Attributes<'_>, _id: &Id) {}
    fn on_search_exit(&self, _id: &Id) {}

    fn on_search_with_depth_enter(&self, _attrs: &Attributes<'_>, _id: &Id) {}
    fn on_search_with_depth_exit(&self, _id: &Id) {}
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
        let _ = event;
        let _ = ctx;
    }
}
