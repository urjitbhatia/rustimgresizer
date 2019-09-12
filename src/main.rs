#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate image;

#[get("/status")]
fn status() -> &'static str {
    "OK"
}

use image::{GenericImageView, ImageError, ImageOutputFormat};
use rocket::http::ContentType;
use rocket::response::content::Content;
use rocket::response::Stream;
use std::io;


#[get("/resize?<height>&<width>")]
fn resize(
    height: Option<f32>,
    width: Option<f32>,
) -> rocket::response::Content<Stream<&'static [u8]>> {
    let img = image::open("./test.jpg").unwrap();
    let resized = resize_image(img, height, width);
    let r = match resized {
        Ok(rimg) => {
            // let b: &[u8] = &rimg;
            let str = Stream::from(&rimg[..]);
            Content(ContentType::JPEG, str)
            // let str = Stream::from("error".as_bytes());
            // Content(ContentType::Plain, str)
        }
        Err(err) => {
            print!("Error resizing img %{:?}", err);
            let str = Stream::from("error".as_bytes());
            Content(ContentType::Plain, str)
        }
    };

    return r;
}
// #[get("/resize?<height>&<width>")]
// fn resize(
//     height: Option<f32>,
//     width: Option<f32>,
// ) -> rocket::response::Content<Result<Vec<u8>, ImageError>> {
//     let img = image::open("./test.jpg").unwrap();
// Content(ContentType::JPEG, resize_image(img, height, width))
// }

fn resize_image(
    img: image::DynamicImage,
    new_h: Option<f32>,
    new_w: Option<f32>,
) -> Result<Vec<u8>, ImageError> {
    let mut result: Vec<u8> = Vec::new();
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

    scaled.write_to(&mut result, ImageOutputFormat::JPEG(90))?;

    Ok(result)
}

struct Server {
    img: image::DynamicImage,
}

fn main() {
    rocket::ignite()
        .mount("/", routes![status, resize])
        .launch();
}
