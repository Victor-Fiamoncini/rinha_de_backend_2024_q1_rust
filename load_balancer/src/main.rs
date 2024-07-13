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
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    http_client: Client<HttpConnector, Body>,
    load_balancer: Arc<dyn LoadBalancer + Send + Sync>,
}

trait LoadBalancer {
    fn next_server(&self, request: &Request) -> String;
}

struct RoundRobinBalancer {
    addresses: Vec<&'static str>,
    request_counter: Arc<AtomicUsize>,
}

impl LoadBalancer for RoundRobinBalancer {
    fn next_server(&self, _: &Request) -> String {
        let count = self.request_counter.fetch_add(1, Ordering::Relaxed);

        self.addresses[count % self.addresses.len()].to_string()
    }
}

struct RinhaAccountBalancer {
    addresses: Vec<&'static str>,
}

impl LoadBalancer for RinhaAccountBalancer {
    fn next_server(&self, request: &Request) -> String {
        let path = request.uri().path();
        let hash = {
            let mut hasher = DefaultHasher::new();

            path.hash(&mut hasher);

            hasher.finish() as usize
        };

        self.addresses[hash % self.addresses.len()].to_string()
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:9999").await.unwrap();
    let http_client = Client::builder(TokioExecutor::new()).build_http::<Body>();
    let addresses = ["0.0.0.0:9997", "0.0.0.0:9998"];

    #[allow(unused)]
    let round_robin = RoundRobinBalancer {
        addresses: addresses.to_vec(),
        request_counter: Arc::new(AtomicUsize::new(0)),
    };

    #[allow(unused)]
    let rinha_account = RinhaAccountBalancer {
        addresses: addresses.to_vec(),
    };

    let app_state = AppState {
        http_client,
        load_balancer: Arc::new(round_robin),
    };

    let app = proxy.with_state(app_state);

    axum::serve(listener, app).await.unwrap();
}

async fn proxy(
    State(AppState {
        http_client,
        load_balancer,
    }): State<AppState>,
    mut request: Request,
) -> impl IntoResponse {
    let address = load_balancer.next_server(&request);

    *request.uri_mut() = {
        let uri = request.uri();
        let mut parts = uri.clone().into_parts();

        parts.authority = Authority::from_str(address.as_str()).ok();
        parts.scheme = Some(Scheme::HTTP);

        Uri::from_parts(parts).unwrap()
    };

    match http_client.request(request).await {
        Ok(res) => Ok(res),
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    }
}
