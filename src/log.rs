// Copyright 2022 Sean Gillespie.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{
    fmt::Arguments,
    fs::File,
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Mutex,
    },
};

#[repr(u8)]
#[derive(PartialEq, PartialOrd, Eq, Debug)]
pub enum LogLevel {
    Debug = 3,
    Info = 2,
    Warn = 1,
    Always = 0,
}

struct Logger {
    enabled: AtomicBool,
    level: AtomicU8,
    file: Mutex<Option<File>>,
}

static LOGGER: Logger = Logger {
    enabled: AtomicBool::new(false),
    file: Mutex::new(None),
    level: AtomicU8::new(LogLevel::Always as u8),
};

pub fn set_file(name: &str) -> io::Result<()> {
    let file = File::options().append(true).open(name)?;
    let mut logger_file = LOGGER.file.lock().unwrap();
    if let Some(old_file) = logger_file.replace(file) {
        old_file.sync_all()?;
    }
    Ok(())
}

pub fn enable() {
    LOGGER.enabled.store(true, Ordering::Release);
}

pub fn disable() {
    LOGGER.enabled.store(false, Ordering::Release);
}

pub fn set_level(level: LogLevel) {
    LOGGER.level.store(level as u8, Ordering::Release);
}

pub fn log(level: LogLevel, args: Arguments<'_>) {
    if LOGGER.enabled.load(Ordering::Acquire) && LOGGER.level.load(Ordering::Acquire) >= level as u8
    {
        let mut file = LOGGER.file.lock().unwrap();
        if let Some(ref mut file) = *file {
            let _ = writeln!(file, "{}", args);
        }
    }
}
