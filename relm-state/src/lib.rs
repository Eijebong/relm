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

//! This crate provide the non-GUI part of relm:
//! Basic component and message connection methods.

#![cfg_attr(feature = "use_impl_trait", feature(conservative_impl_trait))]
#![warn(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results,
)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate may;
extern crate relm_core;

mod into;
mod macros;

use std::time::SystemTime;

pub use relm_core::EventStream;

pub use into::{IntoOption, IntoPair};

macro_rules! relm_connect {
    ($_self:expr, $to_stream:expr, $success_callback:expr, $failure_callback:expr) => {{
        let event_stream = $_self.stream.clone();
        let fail_event_stream = $_self.stream.clone();
        let stream = $to_stream.to_stream();
        stream.map_err(move |error| {
            fail_event_stream.emit($failure_callback(error));
            ()
        })
            .for_each(move |result| {
                event_stream.emit($success_callback(result));
                Ok::<(), STREAM::Error>(())
            }
            .map_err(|_| ()))
    }};
}

macro_rules! relm_connect_ignore {
    ($_self:expr, $to_stream:expr, $success_callback:expr) => {{
        let event_stream = $_self.stream.clone();
        let stream = $to_stream.to_stream();
        stream.map_err(|_| ())
            .for_each(move |result| {
                event_stream.emit($success_callback(result));
                Ok::<(), STREAM::Error>(())
            }
            .map_err(|_| ()))
    }};
}

/// Handle connection of futures to send messages to the [`update()`](trait.Update.html#tymethod.update) method.
pub struct Relm<UPDATE: Update> {
    stream: EventStream<UPDATE::Msg>,
}

impl<UPDATE: Update> Clone for Relm<UPDATE> {
    fn clone(&self) -> Self {
        Relm {
            stream: self.stream.clone(),
        }
    }
}

impl<UPDATE: Update> Relm<UPDATE> {
    /// Create a new relm stream handler.
    pub fn new(stream: EventStream<UPDATE::Msg>) -> Self {
        Relm {
            stream,
        }
    }

    /// Get the event stream of this stream.
    /// This is used internally by the library.
    pub fn stream(&self) -> &EventStream<UPDATE::Msg> {
        &self.stream
    }
}

/// Trait for a basic (non-widget) component.
/// A component has a model (data) associated with it and can mutate it when it receives a message
/// (in the `update()` method).
pub trait Update
    where Self: Sized,
          Self::Msg: DisplayVariant,
{
    /// The type of the model.
    type Model;
    /// The type of the parameter of the model() function used to initialize the model.
    type ModelParam: Sized;
    /// The type of the messages sent to the [`update()`](trait.Update.html#tymethod.update) method.
    type Msg;

    /// Create the initial model.
    fn model(relm: &Relm<Self>, param: Self::ModelParam) -> Self::Model;

    /// Connect the subscriptions.
    /// Subscriptions are `Future`/`Stream` that are spawn when the object is created.
    fn subscriptions(&mut self, _relm: &Relm<Self>) {
    }

    /// Method called when a message is received from an event.
    fn update(&mut self, event: Self::Msg);
}

/// Trait for an `Update` object that can be created directly.
/// This is useful for non-widget component.
pub trait UpdateNew: Update {
    /// Create a new component.
    fn new(_relm: &Relm<Self>, _model: Self::Model) -> Self;
}

/// Format trait for enum variants.
///
/// `DisplayVariant` is similar to `Debug`, but only works on enum and does not list the
/// variants' parameters.
///
/// This is used internally by the library.
pub trait DisplayVariant {
    /// Formats the current variant of the enum.
    fn display_variant(&self) -> &'static str;
}

impl DisplayVariant for () {
    fn display_variant(&self) -> &'static str {
        ""
    }
}

/// Create a bare component, i.e. a component only implementing the Update trait, not the Widget
/// trait.
pub fn execute<UPDATE>(model_param: UPDATE::ModelParam) -> EventStream<UPDATE::Msg>
where UPDATE: Send + Update + UpdateNew + 'static,
      UPDATE::Msg: Send,
{
    let stream = EventStream::new();

    let relm = Relm::new(stream.clone());
    let model = UPDATE::model(&relm, model_param);
    let component = UPDATE::new(&relm, model);

    init_component::<UPDATE>(&stream, component, &relm);
    stream
}

/// Initialize a component by creating its subscriptions and dispatching the messages from the
/// stream.
pub fn init_component<UPDATE>(stream: &EventStream<UPDATE::Msg>, mut component: UPDATE, relm: &Relm<UPDATE>)
    where UPDATE: Send + Update + 'static,
          UPDATE::Msg: DisplayVariant + Send + 'static,
{
    let stream = stream.clone();
    component.subscriptions(relm);
    go!(move || {
        loop {
            match stream.get_event() {
                Ok(event) => update_component(&mut component, event),
                Err(_) => break, // Stream was disconnected.
            }
        }
    });
}

fn update_component<COMPONENT>(component: &mut COMPONENT, event: COMPONENT::Msg)
    where COMPONENT: Update,
{
    if cfg!(debug_assertions) {
        let time = SystemTime::now();
        let debug = event.display_variant();
        let debug =
            if debug.len() > 100 {
                format!("{}…", &debug[..100])
            }
            else {
                debug.to_string()
            };
        component.update(event);
        if let Ok(duration) = time.elapsed() {
            let ms = duration.subsec_nanos() as u64 / 1_000_000 + duration.as_secs() * 1000;
            if ms >= 200 {
                warn!("The update function was slow to execute for message {}: {}ms", debug, ms);
            }
        }
    }
    else {
        component.update(event)
    }
}
