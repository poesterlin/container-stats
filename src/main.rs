//! Stream stats for all running Docker containers asynchronously
#![type_length_limit = "2097152"]

mod api;
use api::collect_all_stats;

mod leptos_axum;
mod websocket;
use leptos_axum::LeptosHtml;

use axum::{
    extract::{ws::WebSocketUpgrade, Extension, Query},
    response::IntoResponse,
    routing::get,
    Router,
};
use bollard::Docker;
use leptos::view;
use serde::Deserialize;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing::*;
use websocket::{handle_socket, WsState};

#[derive(Debug, Deserialize)]
struct Params {
    sort_key: Option<String>,
}

impl Default for Params {
    fn default() -> Self {
        Params { sort_key: None }
    }
}

async fn index(
    Extension(state): Extension<Arc<WsState>>,
    Query(params): Query<Params>,
) -> LeptosHtml {
    let sort_key = params.sort_key.clone().unwrap_or_default().into();

    let docker = state.docker.lock().await;
    let stats: Vec<api::ContainerStats> = collect_all_stats(&docker, sort_key).await;

    return view! {
        <html lang="en">
            <head>
                <title>Container Stats</title>
                <meta charset="UTF-8"></meta>
                <meta name="viewport" content="width=device-width, initial-scale=1"></meta>
                <link href="/assets/index.css" rel="stylesheet"></link>
                <script src="/assets/update.js"></script>
            </head>
            <body>
                <h1>Container Stats</h1>
            <table>
                <thead>
                    <tr>
                        <th><a href="?sort_key=name">Container Name</a></th>
                        <th><a href="?sort_key=memory">Memory Usage</a></th>
                        <th><a href="?sort_key=cpu">CPU Usage</a></th>
                    </tr>
                </thead>
                <tbody>
                    {stats.into_iter()
                        .map(|stat| view! {
                            <tr id={stat.id}>
                                <td>{ stat.name }</td>
                                <td>{ stat.memory_usage }</td>
                                <td>{ stat.cpu_usage }</td>
                            </tr>
                        })
                        .collect::<Vec<_>>()}
                </tbody>
            </table>
        </body>
    </html>
    }
    .into();
}

#[tokio::main]
async fn main() {
    let docker_connection = Docker::connect_with_socket_defaults();

    let docker = match docker_connection {
        Ok(docker) => docker,
        Err(e) => {
            eprintln!("Error connecting to Docker: {}", e);
            return ();
        }
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/ws", get(ws_handler))
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(Extension(Arc::new(WsState::new(docker))));

    println!("Listening on: http://localhost:42069");

    axum::Server::bind(&"0.0.0.0:42069".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<WsState>>,
) -> impl IntoResponse {
    debug!("New Websocket Connection");
    ws.on_upgrade(|socket| handle_socket(socket, state))
}
