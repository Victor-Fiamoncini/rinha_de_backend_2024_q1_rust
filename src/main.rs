use axum::{
    handler::Handler,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use std::collections::HashMap;
use time::OffsetDateTime;

#[derive(Default, Clone)]
struct Account {
    balance: i64,
    limit: i64,
    transactions: Vec<Transaction>,
}

impl Account {
    pub fn with_limit(limit: i64) -> Self {
        Account {
            limit,
            ..Default::default()
        }
    }
}

#[derive(Clone)]
enum TransactionType {
    Credit,
    Debit,
}

#[derive(Clone)]
struct Transaction {
    value: i64,
    kind: TransactionType,
    description: String,
    created_at: OffsetDateTime,
}

async fn health() -> impl IntoResponse {
    Html("Server is alive!")
}

async fn create_transaction() -> impl IntoResponse {
    "Created transaction"
}

async fn view_account() -> impl IntoResponse {
    "Client account"
}

#[tokio::main]
async fn main() {
    let accounts = HashMap::<u8, Account>::from_iter([
        (1, Account::with_limit(100_000)),
        (2, Account::with_limit(80_000)),
        (3, Account::with_limit(1_000_000)),
        (4, Account::with_limit(10_000_000)),
        (5, Account::with_limit(500_000)),
    ]);

    let app = Router::new()
        .route("/health", get(health))
        .route("/clientes/:id/transacoes", post(create_transaction))
        .route("/clientes/:id/extrato", get(view_account))
        .with_state(accounts);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
