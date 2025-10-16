
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use clap::Parser;
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::RwLock};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Port to bind to
    #[clap(short, long, default_value_t = 3000)]
    port: u16,
}

type Store = Arc<RwLock<HashMap<String, Value>>>;

#[derive(Serialize)]
struct UriResponse {
    uri: String,
}

async fn post_key(
    Path(key): Path<String>,
    State(store): State<Store>,
    Json(value): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let mut db = store.write().await;
    if db.contains_key(&key) {
        let body = serde_json::json!({ "uri": format!("/keys/{}", urlencoding::encode(&key)) });
        return Err((StatusCode::CONFLICT, Json(body)));
    }
    db.insert(key.clone(), value);
    let uri = format!("/keys/{}", urlencoding::encode(&key));
    let res = UriResponse { uri };
    Ok((StatusCode::CREATED, Json(res)))
}

async fn get_key(
    Path(key): Path<String>,
    State(store): State<Store>,
) -> Result<Json<Value>, (StatusCode, Json<serde_json::Value>)> {
    let db = store.read().await;
    match db.get(&key) {
        Some(value) => Ok(Json(value.clone())),
        None => {
            let body = serde_json::json!({ "error": "Key not found" });
            Err((StatusCode::NOT_FOUND, Json(body)))
        }
    }
}

async fn put_key(
    Path(key): Path<String>,
    State(store): State<Store>,
    Json(value): Json<Value>,
) -> Result<Json<UriResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mut db = store.write().await;
    if !db.contains_key(&key) {
        let body = serde_json::json!({ "uri": format!("/keys/{}", urlencoding::encode(&key)) });
        return Err((StatusCode::NOT_FOUND, Json(body)));
    }
    db.insert(key.clone(), value);
    let uri = format!("/keys/{}", urlencoding::encode(&key));
    Ok(Json(UriResponse { uri }))
}

async fn delete_key(
    Path(key): Path<String>,
    State(store): State<Store>,
) -> Result<Json<UriResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mut db = store.write().await;
    if !db.contains_key(&key) {
        let body = serde_json::json!({ "uri": format!("/keys/{}", urlencoding::encode(&key)) });
        return Err((StatusCode::NOT_FOUND, Json(body)));
    }
    db.remove(&key);
    let uri = format!("/keys/{}", urlencoding::encode(&key));
    Ok(Json(UriResponse { uri }))
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let store: Store = Arc::new(RwLock::new(HashMap::new()));

    let app = Router::new()
        .route("/keys/{key}", post(post_key))
        .route("/keys/{key}", get(get_key))
        .route("/keys/{key}", put(put_key))
        .route("/keys/{key}", delete(delete_key))
        .with_state(store);

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    println!("🔑 Key server running on http://{}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}