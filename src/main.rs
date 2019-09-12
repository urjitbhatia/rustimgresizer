#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate image;

#[get("/status")]
fn status() -> &'static str {
    "OK"
}

use pipe::{pipe, PipeReader, PipeWriter};
use std::fs::File;
use std::io::Write;
use std::{io, thread};

use image::{GenericImageView, ImageError, ImageOutputFormat};
use rocket::http::ContentType;
use rocket::response::content::Content;
use rocket::response::Stream;

/// Starts the given function in a thread and return a reader of the generated data
fn image_piper<F: Send + 'static>(writefn: F) -> PipeReader
where
    F: FnOnce(PipeWriter) -> std::result::Result<(), ImageError>,
{
    let (reader, writer) = pipe();
    thread::spawn(move || writefn(writer));
    reader
}
/// Starts the given function in a thread and return a reader of the generated data
fn io_piper<F: Send + 'static>(writefn: F) -> PipeReader
where
    F: FnOnce(PipeWriter) -> io::Result<()>,
{
    let (reader, writer) = pipe();
    thread::spawn(move || writefn(writer));
    reader
}

#[get("/resize", rank = 1)]
fn noresize() -> rocket::response::Content<Stream<File>> {
    println!("using resize2");
    let f = File::open("./test.jpg").unwrap();
    return Content(ContentType::JPEG, Stream::from(f));
}

#[get("/resize?<height>&<width>", rank = 2)]
fn resize(
    height: Option<f32>,
    width: Option<f32>,
) -> rocket::response::Content<Stream<PipeReader>> {
    let img = image::open("./test.jpg").unwrap();

    let resized = resize_image(img, height, width);
    let r = match resized {
        Ok(rimg) => {
            let pp = image_piper(move |mut w| rimg.write_to(&mut w, ImageOutputFormat::JPEG(90)));
            let str = Stream::from(pp);
            Content(ContentType::JPEG, str)
        }
        Err(err) => {
            print!("Error resizing img %{:?}", err);
            Content(
                ContentType::Plain,
                Stream::from(io_piper(|mut w| w.write_all(b"error"))),
            )
        }
    };

    return r;
}

fn resize_image(
    img: image::DynamicImage,
    new_h: Option<f32>,
    new_w: Option<f32>,
) -> Result<image::DynamicImage, ImageError> {
    let old_h = img.height() as f32;
    let old_w = img.width() as f32;

    let scaled = match (new_h, new_w) {
        (None, None) => {
            println!("Using old h,w");
            img
        }
        (Some(h), Some(w)) => {
            println!("Using custom h,w");
            img.resize_exact(h as u32, w as u32, image::FilterType::Lanczos3)
        }
        (None, Some(w)) => {
            println!("Using custom w");
            let ratio = old_h / old_w;
            img.resize_exact(
                w as u32,
                (w * ratio).floor() as u32,
                image::FilterType::Lanczos3,
            )
        }
        (Some(h), None) => {
            println!("Using custom h");
            let ratio = old_h / old_w;
            img.resize_exact(
                h as u32,
                (h * ratio).floor() as u32,
                image::FilterType::Lanczos3,
            )
        }
    };
    Ok(scaled)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![status, noresize, resize])
        .launch();
}
