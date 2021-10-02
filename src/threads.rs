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
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Condvar, Mutex, Once, RwLock,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tracing::Level;

use crate::Position;

/// External interface to the thread pool.
pub struct Threads {
    main_thread: MainThread,
    worker_threads: Vec<WorkerThread>,
}

impl Threads {
    fn new() -> Threads {
        Threads {
            main_thread: MainThread::new(),
            worker_threads: vec![WorkerThread::new()],
        }
    }

    /// Gets a reference to the main thread, for the purposes of sending messages to it.
    pub fn main_thread(&self) -> &MainThread {
        &self.main_thread
    }

    pub fn worker_threads(&self) -> &[WorkerThread] {
        &self.worker_threads
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
    Search,
    Stop,
}

enum Response {
    Ping,
    Shutdown,
    Stop,
}

pub struct MainThread {
    handle: JoinHandle<()>,
    request_tx: SyncSender<Request>,
    response_rx: Receiver<Response>,
    position: RwLock<Position>,
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
            position: RwLock::new(Position::new()),
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

    pub fn search(&self) {
        self.request_tx
            .send(Request::Search)
            .expect("search failed to send on request tx");
    }

    pub fn stop(&self) {
        self.request_tx
            .send(Request::Stop)
            .expect("stop failed to send on request tx");
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

    pub fn set_position(&self, pos: Position) {
        *self.position.write().unwrap() = pos;
    }
}

fn thread_loop(rx: Receiver<Request>, tx: SyncSender<Response>) {
    let _span = tracing::span!(Level::INFO, "main_thread").entered();
    let loop_result: Result<(), mpsc::SendError<Response>> = try {
        tracing::debug!("entering main loop");
        while let Ok(req) = rx.recv() {
            match req {
                Request::Ping => tx.send(Response::Ping)?,
                Request::Shutdown => {
                    tx.send(Response::Shutdown)?;
                    return;
                }
                Request::Search => {
                    tracing::debug!("sending start signal to workers");
                    for worker in get().worker_threads() {
                        worker.start();
                    }
                }
                Request::Stop => {
                    for worker in get().worker_threads() {
                        worker.stop();
                    }
                }
            }
        }
    };

    loop_result.expect("failed to send response to calling thread");
}

pub struct WorkerThread {
    handle: JoinHandle<()>,
    idle_lock: Mutex<bool>,
    idle_cv: Condvar,
    stop_flag: AtomicBool,
    shutdown_flag: AtomicBool,
}

impl WorkerThread {
    pub fn new() -> WorkerThread {
        let handle = thread::Builder::new()
            .name("a4 worker thread".into())
            .spawn(|| {
                THREAD_KIND.with(|kind| *kind.borrow_mut() = ThreadIdentifier::WorkerThread(0));
                worker_thread_loop()
            })
            .expect("failed to spawn main thread");

        WorkerThread {
            handle,
            idle_lock: Mutex::new(true),
            idle_cv: Condvar::new(),
            stop_flag: AtomicBool::new(false),
            shutdown_flag: AtomicBool::new(false),
        }
    }

    pub fn shutdown(self) {
        self.stop_flag.store(true, Ordering::Release);
        self.shutdown_flag.store(true, Ordering::Release);
        self.start();
        self.handle.join().unwrap();
    }

    pub fn start(&self) {
        let mut idle = self.idle_lock.lock().unwrap();
        *idle = false;
        self.idle_cv.notify_one();
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Release);
    }
}

fn worker_thread_loop() {
    let (id, thread) = current().unwrap_worker();
    let _span = tracing::span!(Level::DEBUG, "worker_thread", id).entered();
    tracing::debug!("entering main loop");
    loop {
        tracing::debug!("waiting for start signal");
        let idle = thread.idle_lock.lock().unwrap();
        let mut idle = thread.idle_cv.wait_while(idle, |idle| *idle).unwrap();
        if thread.shutdown_flag.load(Ordering::Acquire) {
            tracing::debug!("received shutdown signal, terminating");
            return;
        }

        tracing::debug!("worker becoming active");
        while !thread.stop_flag.load(Ordering::Acquire) {
            println!("doing work!");
            thread::sleep(Duration::from_secs(1));
        }

        tracing::debug!("worker received stop signal");
        thread.stop_flag.store(false, Ordering::Release);
        *idle = true;
    }
}

enum ThreadIdentifier {
    MainThread,
    WorkerThread(usize),
    Unknown,
}

enum ThreadKind {
    Main(&'static MainThread),
    Worker(usize, &'static WorkerThread),
    Unknown,
}

impl ThreadKind {
    pub fn unwrap_main(self) -> &'static MainThread {
        match self {
            ThreadKind::Main(thread) => thread,
            ThreadKind::Worker(_, _) => panic!("unwrap_main() called on worker thread"),
            ThreadKind::Unknown => panic!("unwrap_main() called on unknown thread"),
        }
    }

    pub fn unwrap_worker(self) -> (usize, &'static WorkerThread) {
        match self {
            ThreadKind::Main(_) => panic!("unwrap_worker() called on main thread"),
            ThreadKind::Worker(id, thread) => (id, thread),
            ThreadKind::Unknown => panic!("unwrap_main() called on unknown thread"),
        }
    }
}

thread_local! {
    static THREAD_KIND: RefCell<ThreadIdentifier> = RefCell::new(ThreadIdentifier::Unknown);
}

fn current() -> ThreadKind {
    let threads = get();
    THREAD_KIND.with(|kind| match *kind.borrow() {
        ThreadIdentifier::MainThread => ThreadKind::Main(threads.main_thread()),
        ThreadIdentifier::WorkerThread(id) => ThreadKind::Worker(id, &threads.worker_threads()[id]),
        ThreadIdentifier::Unknown => ThreadKind::Unknown,
    })
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
