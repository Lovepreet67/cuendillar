use std::ops::{Deref, DerefMut};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Instant;

use crate::database::db_engine::errors::EngineError;

pub struct InstrumentedRwLock<T> {
    name: &'static str,
    inner: RwLock<T>,
}

impl<T> InstrumentedRwLock<T> {
    pub fn new(name: &'static str, value: T) -> Self {
        Self {
            name,
            inner: RwLock::new(value),
        }
    }

    pub fn read(&self, message: &'static str) -> Result<InstrumentedReadGuard<'_, T>, ()> {
        let start = Instant::now();
        let guard = self.inner.read().unwrap();
        let waited = start.elapsed();
        let tag = if waited.as_micros() > 50 {
            "Important"
        } else {
            "Normal"
        };

        println!(
            "[LOCK ACQUIRED] {} name={} type=READ wait={:?}, {}",
            tag, self.name, waited, message
        );

        Ok(InstrumentedReadGuard {
            name: self.name,
            acquired_at: Instant::now(),
            guard,
        })
    }

    pub fn write(&self, message: &'static str) -> Result<InstrumentedWriteGuard<'_, T>, ()> {
        let start = Instant::now();
        let guard = self.inner.write().unwrap();
        let waited = start.elapsed();
        let tag = if waited.as_micros() > 50 {
            "Important"
        } else {
            "Normal"
        };

        println!(
            "[LOCK ACQUIRED] {} name={} type=WRITE wait={:?}, {}",
            tag, self.name, waited, message
        );

        Ok(InstrumentedWriteGuard {
            name: self.name,
            acquired_at: Instant::now(),
            guard,
        })
    }
}

pub struct InstrumentedReadGuard<'a, T> {
    name: &'static str,
    acquired_at: Instant,
    guard: RwLockReadGuard<'a, T>,
}

impl<'a, T> Deref for InstrumentedReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> Drop for InstrumentedReadGuard<'a, T> {
    fn drop(&mut self) {
        let held = self.acquired_at.elapsed();
        let tag = if held.as_micros() > 50 {
            "Important"
        } else {
            "Normal"
        };
        println!(
            "[LOCK RELEASED] {} name={} type=READ held={:?}",
            tag, self.name, held
        );
    }
}

pub struct InstrumentedWriteGuard<'a, T> {
    name: &'static str,
    acquired_at: Instant,
    guard: RwLockWriteGuard<'a, T>,
}

impl<'a, T> Deref for InstrumentedWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> DerefMut for InstrumentedWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl<'a, T> Drop for InstrumentedWriteGuard<'a, T> {
    fn drop(&mut self) {
        let held = self.acquired_at.elapsed();
        let tag = if held.as_micros() > 50 {
            "Important"
        } else {
            "Normal"
        };
        println!(
            "[LOCK RELEASED] {} name={} type=WRITE held={:?}",
            tag, self.name, held
        );
    }
}

impl From<()> for EngineError {
    fn from(_value: ()) -> Self {
        return EngineError::General;
    }
}
