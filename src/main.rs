use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::sync::RwLock;

#[derive(Clone, Serialize)]
struct RingBuffer<T>(VecDeque<T>);

impl<T> Default for RingBuffer<T> {
    fn default() -> Self {
        Self::with_capacity(10)
    }
}

impl<T> RingBuffer<T> {
    fn with_capacity(capacity: usize) -> Self {
        Self(VecDeque::with_capacity(capacity))
    }

    fn push(&mut self, item: T) {
        if self.0.len() > self.0.capacity() {
            self.0.pop_back();
            self.0.push_front(item);
        } else {
            self.0.push_front(item);
        }
    }
}

#[derive(Default, Clone)]
struct Account {
    balance: i64,
    limit: i64,
    transactions: RingBuffer<Transaction>,
}

struct Description(String);

impl TryForm<String> for Description {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {}
}

impl Account {
    pub fn with_limit(limit: i64) -> Self {
        Account {
            limit,
            ..Default::default()
        }
    }

    pub fn transact(&mut self, transaction: Transaction) -> Result<(), &'static str> {
        match transaction.kind {
            TransactionType::Credit => {
                self.balance += transaction.value;
                self.transactions.push(transaction);

                Ok(())
            }
            TransactionType::Debit => {
                if self.balance + self.limit >= self.limit {
                    self.balance -= transaction.value;
                    self.transactions.push(transaction);

                    Ok(())
                } else {
                    Err("The amount debited will exceed the client's limit")
                }
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
enum TransactionType {
    #[serde(rename = "c")]
    Credit,
    #[serde(rename = "d")]
    Debit,
}

#[derive(Clone, Serialize, Deserialize)]
struct Transaction {
    #[serde(rename = "valor")]
    value: i64,
    #[serde(rename = "tipo")]
    kind: TransactionType,
    #[serde(rename = "descricao")]
    description: String,
    #[serde(
        rename = "realizada_em",
        with = "time::serde::rfc3339",
        default = "OffsetDateTime::now_utc"
    )]
    created_at: OffsetDateTime,
}

type AppState = Arc<HashMap<u8, RwLock<Account>>>;

async fn health() -> impl IntoResponse {
    Html("Server is alive!")
}

async fn create_transaction(
    Path(account_id): Path<u8>,
    State(accounts): State<AppState>,
    Json(transaction): Json<Transaction>,
) -> impl IntoResponse {
    match accounts.get(&account_id) {
        Some(account) => {
            let mut account = account.write().await;

            match account.transact(transaction) {
                Ok(()) => Ok(Json(
                    json!({"limite": account.limit, "saldo": account.balance}),
                )),
                Err(_) => Err(StatusCode::UNPROCESSABLE_ENTITY),
            }
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn view_account(
    Path(account_id): Path<u8>,
    State(accounts): State<AppState>,
) -> impl IntoResponse {
    match accounts.get(&account_id) {
        Some(account) => {
            let account = account.read().await;

            Ok(Json(json!({
                "saldo": {
                    "data_extrato": OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
                    "limite": account.limit,
                    "total": account.balance
                },
                "ultimas_transacoes": account.transactions
            })))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[tokio::main]
async fn main() {
    let accounts = HashMap::<u8, RwLock<Account>>::from_iter([
        (1, RwLock::new(Account::with_limit(100_000))),
        (2, RwLock::new(Account::with_limit(80_000))),
        (3, RwLock::new(Account::with_limit(1_000_000))),
        (4, RwLock::new(Account::with_limit(10_000_000))),
        (5, RwLock::new(Account::with_limit(500_000))),
    ]);

    let app = Router::new()
        .route("/health", get(health))
        .route("/clientes/:id/transacoes", post(create_transaction))
        .route("/clientes/:id/extrato", get(view_account))
        .with_state(Arc::new(accounts));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
