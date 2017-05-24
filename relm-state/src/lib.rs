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

extern crate futures;
extern crate futures_glib;
#[macro_use]
extern crate log;
extern crate relm_core;

mod component;
mod into;
mod macros;
mod stream;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::SystemTime;

use futures::{Future, Stream};
use futures::future::Executor;
use futures_glib::MainContext;
pub use relm_core::EventStream;

pub use component::Component;
pub use into::{IntoOption, IntoPair};
use stream::ToStream;

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
    cx: MainContext,
    stream: EventStream<UPDATE::Msg>,
}

impl<UPDATE: Update> Clone for Relm<UPDATE> {
    fn clone(&self) -> Self {
        Relm {
            cx: self.cx.clone(),
            stream: self.stream.clone(),
        }
    }
}

impl<UPDATE: Update> Relm<UPDATE> {
    pub fn new(cx: MainContext, stream: EventStream<UPDATE::Msg>) -> Self {
        Relm {
            cx,
            stream,
        }
    }

    #[cfg(feature = "use_impl_trait")]
    pub fn connect<CALLBACK, FAILCALLBACK, STREAM, TOSTREAM>(&self, to_stream: TOSTREAM,
            success_callback: CALLBACK, failure_callback: FAILCALLBACK) -> impl Future<Item=(), Error=()>
        where CALLBACK: Fn(STREAM::Item) -> UPDATE::Msg + 'static,
              FAILCALLBACK: Fn(STREAM::Error) -> UPDATE::Msg + 'static,
              STREAM: Stream + 'static,
              TOSTREAM: ToStream<STREAM, Item=STREAM::Item, Error=STREAM::Error> + 'static,
    {
        relm_connect!(self, to_stream, success_callback, failure_callback)
    }

    #[cfg(not(feature = "use_impl_trait"))]
    /// Connect a `Future` or a `Stream` called `to_stream` to send the message `success_callback`
    /// in case of success and `failure_callback` in case of failure.
    ///
    /// ## Note
    /// This function does not spawn the future.
    /// You'll usually want to use [`Relm::connect_exec()`](struct.Relm.html#method.connect_exec) to both connect and spawn the future.
    pub fn connect<CALLBACK, FAILCALLBACK, STREAM, TOSTREAM>(&self, to_stream: TOSTREAM, success_callback: CALLBACK,
            failure_callback: FAILCALLBACK) -> Box<Future<Item=(), Error=()>>
        where CALLBACK: Fn(STREAM::Item) -> UPDATE::Msg + 'static,
              FAILCALLBACK: Fn(STREAM::Error) -> UPDATE::Msg + 'static,
              STREAM: Stream + 'static,
              TOSTREAM: ToStream<STREAM, Item=STREAM::Item, Error=STREAM::Error> + 'static,
              UPDATE::Msg: 'static,
    {
        let future = relm_connect!(self, to_stream, success_callback, failure_callback);
        Box::new(future)
    }

    #[cfg(feature = "use_impl_trait")]
    pub fn connect_ignore_err<CALLBACK, STREAM, TOSTREAM>(&self, to_stream: TOSTREAM, success_callback: CALLBACK)
            -> impl Future<Item=(), Error=()>
        where CALLBACK: Fn(STREAM::Item) -> UPDATE::Msg + 'static,
              STREAM: Stream + 'static,
              TOSTREAM: ToStream<STREAM, Item=STREAM::Item, Error=STREAM::Error> + 'static,
    {
        relm_connect_ignore!(self, to_stream, success_callback)
    }

    #[cfg(not(feature = "use_impl_trait"))]
    /// This function is the same as [`Relm::connect()`](struct.Relm.html#method.connect) except it does not take a
    /// `failure_callback`; hence, it ignores the errors.
    pub fn connect_ignore_err<CALLBACK, STREAM, TOSTREAM>(&self, to_stream: TOSTREAM, success_callback: CALLBACK) ->
            Box<Future<Item=(), Error=()>>
        where CALLBACK: Fn(STREAM::Item) -> UPDATE::Msg + 'static,
              STREAM: Stream + 'static,
              TOSTREAM: ToStream<STREAM, Item=STREAM::Item, Error=STREAM::Error> + 'static,
              UPDATE::Msg: 'static,
    {
        Box::new(relm_connect_ignore!(self, to_stream, success_callback))
    }

    /// Connect the future `to_stream` and spawn it on the tokio main loop.
    pub fn connect_exec<CALLBACK, FAILCALLBACK, STREAM, TOSTREAM>(&self, to_stream: TOSTREAM, callback: CALLBACK,
            failure_callback: FAILCALLBACK)
        where CALLBACK: Fn(STREAM::Item) -> UPDATE::Msg + 'static,
              FAILCALLBACK: Fn(STREAM::Error) -> UPDATE::Msg + 'static,
              STREAM: Stream + 'static,
              TOSTREAM: ToStream<STREAM, Item=STREAM::Item, Error=STREAM::Error> + 'static,
              UPDATE::Msg: 'static,
    {
        self.exec(self.connect(to_stream, callback, failure_callback));
    }

    /// Connect the future `to_stream` and spawn it on the tokio main loop, ignoring any error.
    pub fn connect_exec_ignore_err<CALLBACK, STREAM, TOSTREAM>(&self, to_stream: TOSTREAM, callback: CALLBACK)
        where CALLBACK: Fn(STREAM::Item) -> UPDATE::Msg + 'static,
              STREAM: Stream + 'static,
              TOSTREAM: ToStream<STREAM, Item=STREAM::Item, Error=STREAM::Error> + 'static,
              UPDATE::Msg: 'static,
    {
        self.exec(self.connect_ignore_err(to_stream, callback));
    }

    /// Get the main context of this stream.
    pub fn context(&self) -> &MainContext {
        &self.cx
    }

    /// Spawn a future in the tokio event loop.
    pub fn exec<FUTURE: Future<Item=(), Error=()> + 'static>(&self, future: FUTURE) {
        self.cx.execute(future);
    }

    /// Get the event stream of this stream.
    /// This is used internally by the library.
    pub fn stream(&self) -> &EventStream<UPDATE::Msg> {
        &self.stream
    }
}

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

    /// Create a new component.
    fn new(_relm: &Relm<Self>, _model: Self::Model) -> Option<Self> {
        None
    }

    /// Connect the subscriptions.
    /// Subscriptions are `Future`/`Stream` that are spawn when the object is created.
    fn subscriptions(&mut self, _relm: &Relm<Self>) {
    }

    /// Method called when a message is received from an event.
    fn update(&mut self, event: Self::Msg);
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
pub fn execute<UPDATE>(model_param: UPDATE::ModelParam) -> Component<UPDATE>
    where UPDATE: Update + 'static
{
    let cx = MainContext::default(|cx| cx.clone());
    let stream = EventStream::new();

    let relm = Relm::new(cx.clone(), stream.clone());
    let model = UPDATE::model(&relm, model_param);
    let update = UPDATE::new(&relm, model)
        .expect("Update::new() was called for a component that has not implemented this method");

    let component = Component::new(stream, Rc::new(RefCell::new(update)));
    init_component::<UPDATE>(&component, &cx, &relm);
    component
}

pub fn init_component<WIDGET>(component: &Component<WIDGET>, cx: &MainContext, relm: &Relm<WIDGET>)
    where WIDGET: Update + 'static,
          WIDGET::Msg: DisplayVariant + 'static,
{
    let stream = component.stream().clone();
    component.widget_mut().subscriptions(relm);
    let widget = component.widget_rc().clone();
    let event_future = stream.for_each(move |event| {
        let mut widget = widget.borrow_mut();
        update_widget(&mut *widget, event);
        Ok(())
    });
    cx.execute(event_future);
}

fn update_widget<WIDGET>(widget: &mut WIDGET, event: WIDGET::Msg)
    where WIDGET: Update,
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
        widget.update(event);
        if let Ok(duration) = time.elapsed() {
            let ms = duration.subsec_nanos() as u64 / 1_000_000 + duration.as_secs() * 1000;
            if ms >= 200 {
                warn!("The update function was slow to execute for message {}: {}ms", debug, ms);
            }
        }
    }
    else {
        widget.update(event)
    }
}
