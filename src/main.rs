//! Stream stats for all running Docker containers asynchronously
#![type_length_limit = "2097152"]

mod api;
use api::collect_all_stats;

mod leptos_axum;
use leptos_axum::LeptosHtml;

use axum::{routing::get, Router};
use bollard::Docker;
use leptos::view;
use tower_http::services::ServeDir;

async fn index() -> LeptosHtml {
    let docker_connection = Docker::connect_with_socket_defaults();

    let docker = match docker_connection {
        Ok(docker) => docker,
        Err(e) => {
            eprintln!("Error connecting to Docker: {}", e);
            return LeptosHtml::from("Error connecting to Docker".to_string());
        }
    };

    let stats = collect_all_stats(&docker).await;

    return view! {
        <html lang="en">
            <head>
                <title>Container Stats</title>
                <meta charset="UTF-8"></meta>
                <meta name="viewport" content="width=device-width, initial-scale=1"></meta>
                <link href="/assets/index.css" rel="stylesheet"></link>
                <meta http-equiv="refresh" content="5"></meta>
            </head>
            <body>
                <h1>Container Stats</h1>
            <table>
                <thead>
                    <tr>
                        // <th>Container ID</th>
                        <th>Container Name</th>
                        <th>Memory Usage</th>
                        <th>CPU Usage</th>
                    </tr>
                </thead>
                <tbody>
                    {stats.into_iter()
                        .map(|stat| view! {
                            <tr>
                                // <td>{ stat.id }</td>
                                <td>{ stat.name }</td>
                                <td>{ stat.memory_usage }</td>
                                <td>{ format!("{:.2}%", stat.cpu_usage) }</td>
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
    let app = Router::new()
        .route("/", get(index))
        .nest_service("/assets", ServeDir::new("assets"));

    axum::Server::bind(&"0.0.0.0:42069".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
