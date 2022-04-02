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

use std::{
    cell::RefCell,
    lazy::SyncOnceCell,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
        mpsc::{Receiver, SyncSender},
        Condvar, Mutex, RwLock,
    },
    thread,
    time::Duration,
};

use crate::{
    position::Position,
    search::{self, SearchOptions},
};

#[derive(Clone, Default)]
pub struct SearchRequest {
    /// Maximum amount of time to dedicate to this search.
    pub time_limit: Option<Duration>,

    /// Maximum amount of nodes to evaluate.
    pub node_limit: Option<u64>,

    /// Maximum depth to search.
    pub depth: Option<u32>,
}

pub enum Request {
    Search,
    Stop,
}

pub struct MainThread {
    tx: SyncSender<Request>,
    position: RwLock<Option<Position>>,
    search: RwLock<Option<SearchRequest>>,
}

impl MainThread {
    fn new() -> MainThread {
        let (tx, rx) = mpsc::sync_channel(0);
        let _handle = thread::Builder::new()
            .name("a4 main thread".into())
            .spawn(move || {
                main_thread_loop(rx);
            })
            .expect("failed to spawn main thread");

        MainThread {
            tx,
            position: RwLock::new(None),
            search: RwLock::new(None),
        }
    }

    fn position(&self) -> Option<Position> {
        self.position
            .read()
            .expect("failed to acquire position read lock")
            .clone()
    }

    fn search(&self) -> Option<SearchRequest> {
        self.search
            .read()
            .expect("failed to acquire search read lock")
            .clone()
    }

    pub fn set_position(&self, pos: Position) {
        *self
            .position
            .write()
            .expect("failed to acquire position write lock") = Some(pos);
    }

    pub fn set_search(&self, search: SearchRequest) {
        *self
            .search
            .write()
            .expect("failed to acquire search write lock") = Some(search);
    }

    pub fn begin_search(&self) {
        self.tx
            .send(Request::Search)
            .expect("failed to send message to main thread");
    }

    pub fn stop(&self) {
        self.tx
            .send(Request::Stop)
            .expect("failed to send message to main thread");
    }
}

fn main_thread_loop(rx: Receiver<Request>) {
    let _span = tracing::info_span!("main_thread").entered();
    tracing::info!("starting");
    while let Ok(req) = rx.recv() {
        match req {
            Request::Search => {
                tracing::info!("sending start signal to workers");
                for worker in get_worker_threads() {
                    worker.start();
                }
            }
            Request::Stop => {
                tracing::info!("sending stop signal to workers");
                for worker in get_worker_threads() {
                    worker.stop();
                    worker.wait_until_idle()
                }

                tracing::info!("all workers are now idle")
            }
        }
    }
}

pub struct WorkerThread {
    id: usize,
    idle_lock: Mutex<bool>,
    idle_cv: Condvar,
    stop_flag: AtomicBool,
}

impl WorkerThread {
    pub fn new(id: usize) -> WorkerThread {
        WorkerThread {
            id,
            idle_lock: Mutex::new(true),
            idle_cv: Condvar::new(),
            stop_flag: AtomicBool::new(false),
        }
    }

    fn start(&self) {
        let mut idle = self.idle_lock.lock().expect("failed to acquire idle lock");
        *idle = false;
        self.idle_cv.notify_all();
    }

    fn stop(&self) {
        self.stop_flag.store(true, Ordering::Release);
    }

    fn wait_until_idle(&self) {
        tracing::info!("waiting until worker thread {} is idle", self.id);
        let idle = self.idle_lock.lock().expect("failed to acquire idle lock");
        let _idle = self
            .idle_cv
            .wait_while(idle, |idle| !*idle)
            .expect("failed to wait on condvar");
        tracing::info!("worker thread {} is idle", self.id);
    }

    fn thread_loop(&self) {
        let _span = tracing::info_span!("worker_thread", self.id).entered();
        let main_thread = get_main_thread();
        tracing::info!("entering worker loop");
        loop {
            let idle = self.idle_lock.lock().expect("failed to acquire idle lock");
            let mut idle = self
                .idle_cv
                .wait_while(idle, |idle| *idle)
                .expect("failed to wait on condvar");

            tracing::info!("worker becoming active");
            if let Some(search) = main_thread.search() {
                let position = main_thread
                    .position()
                    .expect("search requested with no position?");

                let opts = SearchOptions {
                    time_limit: search.time_limit,
                    node_limit: search.node_limit,
                    hard_stop: Some(&self.stop_flag),
                    depth: search.depth.unwrap_or(10),
                };

                search::search(&position, &opts);

                // The 0th worker thread is special in that it is responsible for printing its search results to stdout.
                if self.id == 0 {
                    tracing::info!("stopping search for other threads");
                    for worker in get_worker_threads() {
                        if worker.id == self.id {
                            continue;
                        }

                        worker.stop();
                        worker.wait_until_idle()
                    }
                }
            } else {
                tracing::warn!("worker going back to sleep due to no search work");
            }

            self.stop_flag.store(false, Ordering::Release);
            *idle = true;
            tracing::info!("worker is idle");
        }
    }
}

pub fn get_main_thread() -> &'static MainThread {
    static MAIN_THREAD: SyncOnceCell<MainThread> = SyncOnceCell::new();

    &MAIN_THREAD.get_or_init(MainThread::new)
}

pub fn get_worker_threads() -> &'static [WorkerThread] {
    static WORKER_THREADS: SyncOnceCell<Vec<WorkerThread>> = SyncOnceCell::new();

    &WORKER_THREADS.get_or_init(|| {
        let mut workers = vec![];
        for id in 0..num_cpus::get() {
            workers.push(WorkerThread::new(id));
        }

        workers
    })
}

thread_local! {
    static WORKER_THREAD_ID: RefCell<Option<usize>> = RefCell::new(None);
}

pub fn get_worker_id() -> Option<usize> {
    WORKER_THREAD_ID.with(|id| *id.borrow())
}

pub fn initialize() {
    let _ = get_main_thread();
    let workers = get_worker_threads();
    for worker in workers {
        thread::Builder::new()
            .name(format!("a4 worker thread #{}", worker.id))
            .spawn(move || {
                WORKER_THREAD_ID.with(|id| *id.borrow_mut() = Some(worker.id));
                worker.thread_loop();
            })
            .expect("failed to spawn worker thread");
    }
}
