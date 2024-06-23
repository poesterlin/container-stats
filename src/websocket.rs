use axum::extract::ws::{Message, WebSocket};
use bollard::Docker;
use futures::{stream::SplitSink, SinkExt, StreamExt};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing::*;

use crate::api::{collect_all_stats, ContainerStats};

pub struct WsState {
    txs: Mutex<HashMap<String, SplitSink<WebSocket, Message>>>,
    pub docker: Mutex<Docker>,
}

impl WsState {
    pub fn new(docker: Docker) -> Self {
        WsState {
            txs: Mutex::new(HashMap::default()),
            docker: Mutex::new(docker),
        }
    }
}

pub async fn handle_socket(socket: WebSocket, state: Arc<WsState>) {
    let (tx, _rx) = socket.split();

    let peer_id = format!("{:p}", &tx);

    let mut txs = state.txs.lock().await;
    txs.insert(peer_id.clone(), tx);

    if txs.len() == 1 {
        tokio::spawn(broadcast_loop(state.clone()));
    }
}

async fn broadcast(state: Arc<WsState>, mut services: Vec<ContainerStats>) -> bool {
    let mut txs = state.txs.lock().await;

    // remove duplicates from the list
    let length_before = services.len();

    services.sort_by(|a, b| a.name.cmp(&b.name));
    services.dedup_by(|a, b| a.id == b.id);

    let length_after = services.len();
    if length_before != length_after {
        println!(
            "Removed {} duplicate services",
            length_before - length_after
        );
    }

    let mut peers_to_remove = Vec::new();
    for (peer_id, tx) in txs.iter_mut() {
        let result = tx
            .send(Message::Text(serde_json::to_string(&services).unwrap()))
            .await;

        if let Err(err) = result {
            warn!("Could not send message: {}", err);
            // Remove the tx from the state
            peers_to_remove.push(peer_id.clone());
        }
    }

    for peer_id in peers_to_remove {
        txs.remove(&peer_id);
    }

    let still_connected = txs.len() > 0;
    still_connected
}

async fn broadcast_loop(state: Arc<WsState>) {
    loop {
        let docker = state.docker.lock().await;
        let stats = collect_all_stats(&docker).await;
        let still_connected = broadcast(state.clone(), stats).await;
        if !still_connected {
            info!("No more connected peers");
            return ();
        }
    }
}
