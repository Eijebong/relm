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

#![feature(conservative_impl_trait, fn_traits, proc_macro, unboxed_closures)]

extern crate gdk;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate hyper;
extern crate json;
#[macro_use]
extern crate may;
#[macro_use]
extern crate relm;
extern crate relm_attributes;
#[macro_use]
extern crate relm_derive;
extern crate simplelog;

use std::io::Read;

use gdk::RGBA;
use gdk_pixbuf::PixbufLoader;
use gtk::{
    ButtonExt,
    ImageExt,
    Inhibit,
    LabelExt,
    OrientableExt,
    StateFlags,
    WidgetExt,
};
use hyper::{Client, Error};
use gtk::Orientation::Vertical;
use relm::{Relm, Widget};
use relm_attributes::widget;
use simplelog::{Config, TermLogger};
use simplelog::LogLevelFilter::Warn;

use self::Msg::*;

const RED: &RGBA = &RGBA { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 };

pub struct Model {
    button_enabled: bool,
    gif_url: String,
    loader: PixbufLoader,
    relm: Relm<Win>,
    topic: String,
    text: String,
}

#[derive(SimpleMsg)]
pub enum Msg {
    DownloadCompleted,
    FetchUrl,
    HttpError(String),
    ImageChunk(Vec<u8>),
    NewGif(Vec<u8>),
    Quit,
}

#[widget]
impl Widget for Win {
    fn model(relm: &Relm<Self>, _: ()) -> Model {
        let topic = "cats";
        Model {
            button_enabled: true,
            gif_url: "waiting.gif".to_string(),
            loader: PixbufLoader::new(),
            relm: relm.clone(),
            topic: topic.to_string(),
            text: topic.to_string(),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            DownloadCompleted => {
                self.model.button_enabled = true;
                self.button.grab_focus();
                self.model.loader.close().unwrap();
                self.image.set_from_pixbuf(self.model.loader.get_pixbuf().as_ref());
                self.model.loader = PixbufLoader::new();
            },
            FetchUrl => {
                self.model.text = String::new();
                // Disable the button because loading 2 images at the same time crashes the pixbuf
                // loader.
                self.model.button_enabled = false;

                let url = format!("http://api.giphy.com/v1/gifs/random?api_key=dc6zaTOxFJmzC&tag={}",
                    self.model.topic);
                let stream = self.model.relm.clone();
                go!(move || {
                    let client = Client::new();
                    let mut response = client.get(&*url)
                        .send()
                        .expect("FetchUrl client");
                    let mut buffer = vec![];
                    response.read_to_end(&mut buffer).expect("FetchUrl read_to_end");
                    stream.stream().emit(NewGif(buffer));
                });
            },
            HttpError(error) => {
                self.model.button_enabled = true;
                self.model.text = format!("HTTP error: {}", error);
                self.label.override_color(StateFlags::NORMAL, RED);
            },
            ImageChunk(chunk) => {
                self.model.loader.loader_write(&chunk).unwrap();
            },
            NewGif(result) => {
                let string = String::from_utf8(result).unwrap();
                let mut json = json::parse(&string).unwrap();
                let url = json["data"]["image_url"].take_string().unwrap();
                let stream = self.model.relm.clone();
                go!(move || {
                    let client = Client::new();
                    let mut response = client.get(&url)
                        .send()
                        .expect("NewGif client");
                    let mut buffer = [0; 4096];
                    let mut size_read = 1;
                    while size_read > 0 {
                        size_read = response.read(&mut buffer).expect("NewGif read");
                        stream.stream().emit(ImageChunk(buffer[..size_read].to_vec()));
                    }
                    stream.stream().emit(DownloadCompleted);
                });
            },
            Quit => gtk::main_quit(),
        }
    }

    view! {
        gtk::Window {
            gtk::Box {
                orientation: Vertical,
                #[name="label"]
                gtk::Label {
                    text: &self.model.text,
                },
                #[name="image"]
                gtk::Image {
                },
                #[name="button"]
                gtk::Button {
                    label: "Load image",
                    sensitive: self.model.button_enabled,
                    clicked => FetchUrl,
                },
            },
            delete_event(_, _) => (Quit, Inhibit(false)),
        }
    }
}

impl Drop for Win {
    fn drop(&mut self) {
        // This is necessary to avoid a GDK warning.
        self.model.loader.close().ok(); // Ignore the error since no data was loaded.
    }
}

fn main() {
    TermLogger::init(Warn, Config::default()).unwrap();
    Win::run(()).unwrap();
}
