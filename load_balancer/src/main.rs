use axum::{
    body::Body,
    extract::{Request, State},
    handler::Handler,
    http::{
        uri::{Authority, Scheme},
        StatusCode, Uri,
    },
    response::IntoResponse,
};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    addresses: Vec<&'static str>,
    http_client: Client<HttpConnector, Body>,
    request_counter: Arc<AtomicUsize>,
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:9999").await.unwrap();

    let app_state = AppState {
        addresses: ["0.0.0.0:9997", "0.0.0.0:9998"].to_vec(),
        http_client: Client::builder(TokioExecutor::new()).build_http::<Body>(),
        request_counter: Arc::new(AtomicUsize::new(0)),
    };

    let app = proxy.with_state(app_state);

    axum::serve(listener, app).await.unwrap();
}

async fn proxy(
    State(AppState {
        addresses,
        http_client,
        request_counter,
    }): State<AppState>,
    mut request: Request,
) -> impl IntoResponse {
    let count = request_counter.fetch_add(1, Ordering::Relaxed);

    *request.uri_mut() = {
        let uri = request.uri();
        let mut parts = uri.clone().into_parts();

        parts.authority = Some(Authority::from_static(addresses[count % addresses.len()]));
        parts.scheme = Some(Scheme::HTTP);

        Uri::from_parts(parts).unwrap()
    };

    match http_client.request(request).await {
        Ok(res) => Ok(res),
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    }
}
