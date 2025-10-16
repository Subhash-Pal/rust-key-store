// src/main.rs

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post, put},
    Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};
use tokio::{net::TcpListener, sync::RwLock};
use reqwest::Client;
use http::Method;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Port to bind to
    #[clap(short, long, default_value_t = 3000)]
    port: u16,

    /// Path to routes JSON file (enables shard-router mode)
    #[clap(long)]
    routes: Option<PathBuf>,
}

#[derive(Serialize)]
struct UriResponse {
    uri: String,
}

// ========================
// Key-Server Mode
// ========================

type Store = Arc<RwLock<HashMap<String, Value>>>;

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

// ========================
// Shard-Router Mode
// ========================

#[derive(Deserialize)]
struct RouteEntry {
    prefix: String,
    target: String,
}

#[derive(Deserialize)]
struct RoutesConfig(Vec<RouteEntry>);

struct ShardRouterState {
    routes: HashMap<String, String>,
    client: Client,
}

// Returns (matched_prefix, backend_key_suffix)
fn find_route(key: &str, routes: &HashMap<String, String>) -> Option<(String, String)> {
    let segments: Vec<&str> = key.split('.').collect();
    // Try longest prefix first (most specific match)
    for i in (1..=segments.len()).rev() {
        let candidate = segments[..i].join(".");
        if routes.contains_key(&candidate) {
            let suffix = if i == segments.len() {
                ".".to_string()
            } else {
                format!(".{}", segments[i..].join("."))
            };
            return Some((candidate, suffix));
        }
    }
    None
}

async fn route_and_proxy(
    method: Method,
    full_key: String,
    routes: &HashMap<String, String>,
    client: &Client,
    body: Option<Value>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let (prefix, backend_key) = find_route(&full_key, routes)
        .ok_or_else(|| {
            let body = serde_json::json!({ "error": "No route found for key" });
            (StatusCode::NOT_FOUND, Json(body))
        })?;

    let backend_url = routes.get(&prefix).unwrap(); // safe: `find_route` only returns existing keys
    let encoded_backend_key = urlencoding::encode(&backend_key);
    let url = format!("{}/keys/{}", backend_url.trim_end_matches('/'), encoded_backend_key);

    let req_builder = match method {
        Method::POST => client.post(&url),
        Method::GET => client.get(&url),
        Method::PUT => client.put(&url),
        Method::DELETE => client.delete(&url),
        _ => unreachable!(),
    };

    let request = if let Some(val) = body {
        req_builder.json(&val)
    } else {
        req_builder
    };

    let res = request.send().await
        .map_err(|e| {
            eprintln!("Proxy error to {}: {}", url, e);
            let body = serde_json::json!({ "error": "Upstream key-server unavailable" });
            (StatusCode::BAD_GATEWAY, Json(body))
        })?;

    let status = res.status();
    let mut json_res: serde_json::Value = res.json().await
        .map_err(|_| {
            let body = serde_json::json!({ "error": "Upstream returned invalid JSON" });
            (StatusCode::BAD_GATEWAY, Json(body))
        })?;

    // Rewrite URI to reflect the original key space (not backend's view)
    if json_res.get("uri").is_some() {
        json_res["uri"] = serde_json::json!(format!("/keys/{}", urlencoding::encode(&full_key)));
    }

    let body_bytes = serde_json::to_vec(&json_res).expect("Failed to serialize JSON response");
    let response = Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body_bytes))
        .expect("Failed to build HTTP response");

    Ok(response)
}

async fn router_post(
    Path(key): Path<String>,
    State(state): State<Arc<ShardRouterState>>,
    Json(value): Json<Value>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    route_and_proxy(Method::POST, key, &state.routes, &state.client, Some(value)).await
}

async fn router_get(
    Path(key): Path<String>,
    State(state): State<Arc<ShardRouterState>>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    route_and_proxy(Method::GET, key, &state.routes, &state.client, None).await
}

async fn router_put(
    Path(key): Path<String>,
    State(state): State<Arc<ShardRouterState>>,
    Json(value): Json<Value>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    route_and_proxy(Method::PUT, key, &state.routes, &state.client, Some(value)).await
}

async fn router_delete(
    Path(key): Path<String>,
    State(state): State<Arc<ShardRouterState>>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    route_and_proxy(Method::DELETE, key, &state.routes, &state.client, None).await
}

// ========================
// Main
// ========================

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Some(routes_path) = args.routes {
        // === Shard Router Mode ===
        let routes_content = std::fs::read_to_string(&routes_path)
            .unwrap_or_else(|_| panic!("Failed to read routes file: {:?}", routes_path));

        let RoutesConfig(route_entries) = serde_json::from_str(&routes_content)
            .expect("Invalid routes.json: expected array of { \"prefix\": \"...\", \"target\": \"...\" }");

        let mut routes = HashMap::new();
        for entry in route_entries {
            if entry.prefix.is_empty() {
                panic!("Empty prefix is not allowed in routes");
            }
            if routes.insert(entry.prefix, entry.target).is_some() {
                panic!("Duplicate prefix found in routes configuration");
            }
        }

        let state = Arc::new(ShardRouterState {
            routes,
            client: Client::new(),
        });

        let app = Router::new()
            .route("/keys/{key}", post(router_post))
            .route("/keys/{key}", get(router_get))
            .route("/keys/{key}", put(router_put))
            .route("/keys/{key}", delete(router_delete))
            .with_state(state);

        let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
        println!("🌐 Shard router listening on http://{}", addr);
        let listener = TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    } else {
        // === Key-Server Mode ===
        let store: Store = Arc::new(RwLock::new(HashMap::new()));

        let app = Router::new()
            .route("/keys/{key}", post(post_key))
            .route("/keys/{key}", get(get_key))
            .route("/keys/{key}", put(put_key))
            .route("/keys/{key}", delete(delete_key))
            .with_state(store);

        let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
        println!("🔑 Key server listening on http://{}", addr);
        let listener = TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    }
}