// Copyright 2017-2021 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Thread pool management for a4, for asynchronous and parallel search routines.
//!
//! a4 spawns a number of threads on startup. These are:
//!  1. The main thread, which receives requests from external systems (such as the UCI driver) and coordinates worker
//!     threads to provide an answer to the request,
//!  2. Worker threads, which perform search work as coordinated by the main thread.

#![allow(dead_code)] // Lots of this code will be used elsewhere in time.

use std::{
    sync::{
        mpsc::{self, Receiver, SyncSender},
        Once,
    },
    thread::{self, JoinHandle},
};

/// External interface to the thread pool.
pub struct Threads {
    main_thread: MainThread,
}

impl Threads {
    fn new() -> Threads {
        Threads {
            main_thread: MainThread::new(),
        }
    }

    /// Gets a reference to the main thread, for the purposes of sending messages to it.
    pub fn main_thread(&self) -> &MainThread {
        &self.main_thread
    }
}

static mut THREADS: Option<Threads> = None;
static INIT: Once = Once::new();

/// Initializes the global thread pool.
pub fn initialize() {
    unsafe {
        INIT.call_once(|| THREADS = Some(Threads::new()));
    }
}

/// Retrieves the global thread pool. Panics if the thread pool hasn't been initialized yet.
pub fn get() -> &'static Threads {
    unsafe { THREADS.as_ref().expect("get called before initialize") }
}

enum Request {
    Ping,
    Shutdown,
}

enum Response {
    Ping,
    Shutdown,
}

pub struct MainThread {
    handle: JoinHandle<()>,
    request_tx: SyncSender<Request>,
    response_rx: Receiver<Response>,
}

impl MainThread {
    fn new() -> MainThread {
        let (request_tx, request_rx) = mpsc::sync_channel(0);
        let (response_tx, response_rx) = mpsc::sync_channel(0);
        let handle = thread::Builder::new()
            .name("a4 main thread".into())
            .spawn(|| {
                thread_loop(request_rx, response_tx);
            })
            .expect("failed to spawn main thread");

        MainThread {
            handle,
            request_tx,
            response_rx,
        }
    }

    pub fn ping(&self) -> bool {
        self.request_tx
            .send(Request::Ping)
            .expect("ping failed to send on request tx");
        let _ = self
            .response_rx
            .recv()
            .expect("ping failed to read on request rx");
        true
    }

    pub fn shutdown(self) {
        self.request_tx
            .send(Request::Shutdown)
            .expect("shutdown failed to send on request tx");
        let _ = self
            .response_rx
            .recv()
            .expect("shutdown failed to recv on request rx");
        self.handle.join().expect("failed to join loop thread");
    }
}

fn thread_loop(rx: Receiver<Request>, tx: SyncSender<Response>) {
    let loop_result: Result<(), mpsc::SendError<Response>> = try {
        while let Ok(req) = rx.recv() {
            match req {
                Request::Ping => tx.send(Response::Ping)?,
                Request::Shutdown => {
                    tx.send(Response::Shutdown)?;
                    return;
                }
            }
        }
    };

    loop_result.expect("failed to send response to calling thread");
}

#[cfg(test)]
mod tests {
    use super::MainThread;

    #[test]
    fn setup_shutdown() {
        let thread = MainThread::new();
        thread.shutdown()
    }

    #[test]
    fn ping_pong() {
        let thread = MainThread::new();
        assert!(thread.ping());
        thread.shutdown();
    }
}
