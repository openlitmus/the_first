#![feature(never_type)]

use bytes::Bytes;
use handlebars::Handlebars;
use http_body_util::Full;
use hyper::server::conn::http1::Builder;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use lazy_static::lazy_static;
use rust_embed::RustEmbed;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use tokio::net::TcpListener;

lazy_static! {
    static ref HANDLEBARS: Handlebars<'static> = {
        let mut hbs = Handlebars::new();
        hbs.register_embed_templates_with_extension::<Templates>(".handlebars")
            .unwrap();
        hbs
    };
}

#[derive(RustEmbed)]
#[folder = "templates"]
struct Templates;

#[derive(RustEmbed)]
#[folder = "assets"]
struct Assets;

fn guess_mime<P>(path: P) -> Option<&'static str>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext {
            "css" => Some("text/css"),
            _ => None,
        })
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Box<dyn Error + Send + Sync>> {
    let (parts, _body) = req.into_parts();
    match (&parts.method, parts.uri.path()) {
        (&Method::GET, "/login") => {
            let resp =
                Response::builder().body(Full::new(Bytes::from(HANDLEBARS.render("login", &())?)));
            Ok(resp?)
        }
        (&Method::GET, "/register") => {
            let resp =
                Response::builder().body(Full::new(Bytes::from(HANDLEBARS.render("enroll", &())?)));
            Ok(resp?)
        }
        (&Method::GET, path) => {
            let path = path.trim_start_matches("/");
            if let Some(file) = Assets::get(path) {
                let mut builder = Response::builder();
                if let Some(mime) = guess_mime(&path) {
                    builder = builder.header("Content-Type", mime);
                }
                let resp = builder.body(Full::new(Bytes::from(file.data.into_owned())));
                Ok(resp?)
            } else {
                let resp = Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Full::new(Bytes::from("not found")));
                Ok(resp?)
            }
        }
        _ => {
            let resp = Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Full::new(Bytes::from("method not allowed")));
            Ok(resp?)
        }
    }
}

#[tokio::main]
async fn main() -> Result<!, Box<dyn Error + Send + Sync>> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _addr) = listener.accept().await?;
        let io = TokioIo::new(stream);
        tokio::spawn(async move {
            let http = Builder::new();
            if let Err(err) = http.serve_connection(io, service_fn(handle_request)).await {
                println!("failed to serve connection: {err}");
            }
        });
    }
}
