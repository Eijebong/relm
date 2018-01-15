/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Core primitive types for relm.
//!
//! The primary type is `EventStream`.

#![warn(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results,
)]

extern crate may;

use std::io::Error;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::RecvError;
use std::sync::atomic::{AtomicBool, Ordering};

use may::sync::mpsc::{Receiver, Sender, channel};

struct Observer<MSG> {
    callback: fn(&EventStream<MSG>, &MSG),
    stream: EventStream<MSG>,
}

/// A lock is used to temporarily stop emitting messages.
#[must_use]
pub struct Lock<MSG> {
    stream: Arc<_EventStream<MSG>>,
}

impl<MSG> Drop for Lock<MSG> {
    fn drop(&mut self) {
        self.stream.locked.store(false, Ordering::SeqCst);
    }
}

struct _EventStream<MSG> {
    events: Receiver<MSG>,
    locked: AtomicBool,
    observers: Arc<Mutex<Vec<Arc<Observer<MSG>>>>>,
    sender: Sender<MSG>,
    terminated: AtomicBool,
}

/// A stream of messages to be used for widget/signal communication and inter-widget communication.
pub struct EventStream<MSG> {
    stream: Arc<_EventStream<MSG>>,
}

impl<MSG> Clone for EventStream<MSG> {
    fn clone(&self) -> Self {
        EventStream {
            stream: self.stream.clone(),
        }
    }
}

impl<MSG> EventStream<MSG> {
    /// Create a new event stream.
    pub fn new() -> Self {
        let (tx, rx) = channel();
        EventStream {
            stream: Arc::new(_EventStream {
                events: rx,
                locked: AtomicBool::new(false),
                observers: Arc::new(Mutex::new(vec![])),
                sender: tx,
                terminated: AtomicBool::new(false),
            }),
        }
    }

    /// Close the event stream, i.e. stop processing messages.
    pub fn close(&self) -> Result<(), Error> {
        self.stream.terminated.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Send the `event` message to the stream and the observers.
    pub fn emit(&self, event: MSG) {
        if !self.stream.locked.load(Ordering::SeqCst) {
            let observers = self.stream.observers.lock().expect("lock observers");
            let len = observers.len();
            for i in 0..len {
                let observer = &observers[i];
                let callback = &observer.callback;
                callback(&observers[i].stream, &event);
            }

            self.stream.sender.send(event);
        }
    }

    pub fn get_event(&self) -> Result<MSG, RecvError> {
        self.stream.events.recv()
    }

    /// Lock the stream (don't emit message) until the `Lock` goes out of scope.
    pub fn lock(&self) -> Lock<MSG> {
        self.stream.locked.store(true, Ordering::SeqCst);
        Lock {
            stream: self.stream.clone(),
        }
    }

    fn is_terminated(&self) -> bool {
        self.stream.terminated.load(Ordering::SeqCst)
    }

    /// Add an observer to the event stream.
    /// This callback will be called every time a message is emmited.
    // TODO: maybe require a new MSG parameter.
    pub fn observe(&self, stream: EventStream<MSG>, callback: fn(&EventStream<MSG>, &MSG)) {
        let mut observers = self.stream.observers.lock().expect("lock observers");
        observers.push(Arc::new(Observer {
            callback,
            stream,
        }));
    }
}
