use std::time::Duration;
use zerg::{
    http::{Body, Method, Request},
    UriExt,
};

fn main() {
    let response = zerg::swarm("http://localhost:9999")
        .concurrency(400)
        .threads(8)
        .duration(Duration::from_secs(5))
        .request(|uri| {
            Request::builder()
                .uri(uri.with_path("clientes/1/extrato"))
                .method(Method::GET)
                .body(Body::empty())
                .unwrap()
        })
        .expecting(|res| res.status().is_success())
        .zerg()
        .unwrap();

    println!("{response}");
}
