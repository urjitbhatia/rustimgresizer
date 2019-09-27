#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate image;
// Load the crate
extern crate statsd;

// Import the client object.
use rocket::State;
use statsd::Client;

use pipe::{pipe, PipeReader, PipeWriter};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::{io, thread};

use image::jpeg;
use image::{GenericImageView, ImageError, ImageOutputFormat};
use rocket::http::ContentType;
use rocket::response::content::Content;
use rocket::response::Stream;

struct AppState {
    statsd_client: Client,
    memoized_img: image::DynamicImage,
}

#[get("/status")]
fn status() -> &'static str {
    "OK"
}

/// Starts the given function in a thread and return a reader of the generated data
fn image_piper<F: Send + 'static>(writefn: F) -> BufReader<PipeReader>
where
    F: FnOnce(BufWriter<PipeWriter>) -> std::result::Result<(), ImageError>,
{
    let (reader, writer) = pipe();
    let buf_reader = BufReader::with_capacity(32 * 1024, reader);
    let buf_writer = BufWriter::with_capacity(32 * 1024, writer);
    thread::spawn(move || writefn(buf_writer));
    buf_reader
}

/// Starts the given function in a thread and return a reader of the generated data
fn io_piper<F: Send + 'static>(writefn: F) -> BufReader<PipeReader>
where
    F: FnOnce(BufWriter<PipeWriter>) -> io::Result<()>,
{
    let (reader, writer) = pipe();
    let buf_writer = BufWriter::with_capacity(32 * 1024, writer);
    let buf_reader = BufReader::with_capacity(32 * 1024, reader);
    thread::spawn(move || writefn(buf_writer));
    buf_reader
}

#[get("/resize", rank = 1)]
fn noresize() -> rocket::response::Content<Stream<std::io::BufReader<File>>> {
    println!("using resize2");
    let f = File::open("./test.jpg").unwrap();
    let fin = std::io::BufReader::with_capacity(32 * 1024, f);
    return Content(ContentType::JPEG, Stream::from(fin));
}

#[get("/resize?<height>&<width>", rank = 2)]
fn resize(
    height: Option<f32>,
    width: Option<f32>,
    state: State<AppState>,
) -> Result<rocket::response::Content<Stream<BufReader<PipeReader>>>, io::Error> {
    let img = state.memoized_img.clone();
    let resized = resize_image(img, height, width);
    let r = match resized {
        Ok(rimg) => {
            let bytes = rimg.raw_pixels();
            let mut ww: Vec<u8> = vec![];
            let mut j = jpeg::JPEGEncoder::new_with_quality(&mut ww, 85);

            j.encode(&bytes, rimg.width(), rimg.height(), rimg.color())
                .unwrap();
            let pp = image_piper(move |mut w| rimg.write_to(&mut w, ImageOutputFormat::JPEG(85)));
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

    return Ok(r);
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
    let img = image::open("./test.jpg").unwrap();
    let client = Client::new("127.0.0.1:8125", "rimgresizer").unwrap();
    rocket::ignite()
        .manage(AppState {
            statsd_client: client,
            memoized_img: img,
        })
        .mount("/", routes![status, resize])
        .launch();
}
