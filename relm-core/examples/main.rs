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

extern crate chrono;
extern crate glib;
extern crate gtk;
#[macro_use]
extern crate may;
extern crate relm_core;
extern crate send_cell;

use std::time::Duration;

use chrono::Local;
use glib::MainContext;
use gtk::{
    Button,
    ButtonExt,
    ContainerExt,
    Inhibit,
    Label,
    LabelExt,
    WidgetExt,
    Window,
    WindowType,
};
use gtk::Orientation::Vertical;
use relm_core::EventStream;
use send_cell::SendCell;

use Msg::*;

struct Widgets {
    clock_label: Label,
    counter_label: Label,
}

#[derive(Clone, Debug)]
enum Msg {
    Clock,
    Decrement,
    Increment,
    Quit,
}

struct Model {
    counter: i32,
}

fn main() {
    gtk::init().unwrap();

    let vbox = gtk::Box::new(Vertical, 0);

    let clock_label = Label::new(None);
    vbox.add(&clock_label);

    let plus_button = Button::new_with_label("+");
    vbox.add(&plus_button);

    let counter_label = Label::new("0");
    vbox.add(&counter_label);

    let widgets = Widgets {
        clock_label: clock_label,
        counter_label: counter_label,
    };

    let window = Window::new(WindowType::Toplevel);
    window.add(&vbox);

    let stream = EventStream::new();

    let other_widget_stream = EventStream::new();
    {
        stream.observe(other_widget_stream, |stream, event: &Msg| {
            stream.emit(Quit);
            println!("Event: {:?}", event);
        });
    }

    {
        let stream = stream.clone();
        plus_button.connect_clicked(move |_| {
            stream.emit(Increment);
        });
    }

    let minus_button = Button::new_with_label("-");
    vbox.add(&minus_button);
    {
        let stream = stream.clone();
        minus_button.connect_clicked(move |_| {
            stream.emit(Decrement);
        });
    }

    window.show_all();

    {
        let stream = stream.clone();
        window.connect_delete_event(move |_, _| {
            stream.emit(Quit);
            Inhibit(false)
        });
    }

    let mut model = Model {
        counter: 0,
    };

    fn update(event: Msg, model: &mut Model, widgets: &SendCell<Widgets>) {
        match event {
            Clock => {
                let now = Local::now();
                widgets.get().clock_label.set_text(&now.format("%H:%M:%S").to_string());
            },
            Decrement => {
                model.counter -= 1;
                widgets.get().counter_label.set_text(&model.counter.to_string());
            },
            Increment => {
                model.counter += 1;
                widgets.get().counter_label.set_text(&model.counter.to_string());
            },
            Quit => gtk::main_quit(),
        }
    }

    // TODO
    /*let interval = {
        let interval = Interval::new(Duration::from_secs(1));
        let stream = stream.clone();
        interval.map_err(|_| ()).for_each(move |_| {
            stream.emit(Clock);
            Ok(())
        })
    };*/

    let widgets = SendCell::new(widgets);
    go!(move || {
        loop {
            match stream.get_event() {
                Ok(msg) => {
                    let context = MainContext::default().expect("context");
                    context.invoke(move || {
                        update(msg, &mut model, &widgets);
                    });
                },
                Err(_) => break,
            }
        }
    });

    gtk::main();
}
