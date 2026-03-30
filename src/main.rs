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
use bollard::{
    container::{RestartContainerOptions, StopContainerOptions},
    Docker,
};
use leptos::view;
use serde::Deserialize;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing::*;
use websocket::{handle_socket, WsState};

#[derive(Debug, Deserialize)]
struct Params {
    sort_key: Option<String>,
    restart: Option<String>,
    stop: Option<String>,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            sort_key: None,
            restart: None,
            stop: None,
        }
    }
}

async fn index(
    Extension(state): Extension<Arc<WsState>>,
    Query(params): Query<Params>,
) -> LeptosHtml {
    let sort_key = params.sort_key.clone().unwrap_or_default().into();
    let restart_container_id = params.restart.clone();
    let stop_container_id = params.stop.clone();

    let docker = state.docker.lock().await;

    let mut action_result = None;
    if let Some(container_id) = restart_container_id {
        // remove container_ prefix from id
        let container_id = container_id.trim_start_matches("container_");

        let options = RestartContainerOptions { t: 10 };
        match docker.restart_container(&container_id, Some(options)).await {
            Ok(_) => {
                info!("Restarted container {}", container_id);
                println!("Restarted container {}", container_id);
                action_result = Some("Container restarted");
            }
            Err(e) => {
                error!("Error restarting container {}: {}", container_id, e);
                println!("Error restarting container {}: {}", container_id, e);
                action_result = Some("Error restarting container");
            }
        }
    }

    if let Some(container_id) = stop_container_id {
        // remove container_ prefix from id
        let container_id = container_id.trim_start_matches("container_");

        let options = StopContainerOptions { t: 10 };
        match docker.stop_container(&container_id, Some(options)).await {
            Ok(_) => {
                info!("Stopped container {}", container_id);
                println!("Stopped container {}", container_id);
                action_result = Some("Container stopped");
            }
            Err(e) => {
                error!("Error stopping container {}: {}", container_id, e);
                println!("Error stopping container {}: {}", container_id, e);
                action_result = Some("Error stopping container");
            }
        }
    }

    let stats: Vec<api::ContainerStats> = collect_all_stats(&docker, sort_key).await;

    let result_view = match action_result {
        Some(_) => view! {
            <p id="result" role="status">{action_result.unwrap_or_default()}</p>
        },
        None => view! {
            <p></p>
        },
    };

    return view! {
        <html lang="en">
            <head>
                <title>Container Stats</title>
                <meta charset="UTF-8"></meta>
                <meta name="viewport" content="width=device-width, initial-scale=1"></meta>
                <link href="/assets/index.css" rel="stylesheet"></link>
                <script src="/assets/update.js?v=2"></script>
            </head>
            <body>
                <main class="app-shell">
                    <header class="page-header">
                        <div>
                            <h1>Container Control Center</h1>
                            <p class="subtitle">Live runtime metrics and one-click actions for running containers.</p>
                        </div>
                        <div class="status-strip">
                            <span id="ws-status" class="pill">Connecting</span>
                            <span class="pill pill-muted">Updated <strong id="last-updated">just now</strong></span>
                        </div>
                    </header>

                    {result_view}

                    <section class="table-card">
                        <table>
                            <thead>
                                <tr>
                                    <th><a href="?sort_key=name">Container Name</a></th>
                                    <th>Restart</th>
                                    <th>Stop</th>
                                    <th><a href="?sort_key=memory">Memory Usage</a></th>
                                    <th><a href="?sort_key=cpu">CPU Usage</a></th>
                                    <th><a href="?sort_key=disk_read">Disk Read</a></th>
                                    <th><a href="?sort_key=disk_write">Disk Write</a></th>
                                </tr>
                            </thead>
                            <tbody>
                                {stats.into_iter()
                                    .map(|stat| view! {
                                        <tr id={stat.id.clone()}>
                                            <td class="name-cell">{ stat.name }</td>
                                            <td class="action-cell">
                                                <a class="icon-action" href={"?restart=".to_owned() + &stat.id} title="Restart container">
                                                    <img src="/assets/reload.svg" alt="Restart"></img>
                                                </a>
                                            </td>
                                            <td class="action-cell">
                                                <a class="icon-action icon-action-danger" href={"?stop=".to_owned() + &stat.id} title="Stop container">
                                                    <img src="/assets/stop.svg" alt="Stop"></img>
                                                </a>
                                            </td>
                                            <td class="memory-cell" data-col="memory">{ stat.memory_usage }</td>
                                            <td class="cpu-cell" data-col="cpu">{ stat.cpu_usage }</td>
                                            <td class="disk-read-cell" data-col="disk-read">{ stat.disk_read }</td>
                                            <td class="disk-write-cell" data-col="disk-write">{ stat.disk_write }</td>
                                        </tr>
                                    })
                                    .collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    </section>
                </main>
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
    info!("New Websocket Connection");
    ws.on_upgrade(|socket| handle_socket(socket, state))
}
