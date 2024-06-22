use axum::extract::ws::{Message, WebSocket};
use bollard::Docker;
use futures::{stream::SplitSink, SinkExt, StreamExt};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing::*;

use crate::api::{
    collect_all_stats, get_container_stats_stream, list_running_containers, ContainerStats,
};

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

async fn broadcast(state: Arc<WsState>, services: Vec<ContainerStats>) -> bool {
    let mut txs = state.txs.lock().await;

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
    let mut streams = HashMap::new();

    loop {
        let docker = state.docker.lock().await;
        let stats = list_running_containers(&docker).await;

        for container_id in stats {
            if !streams.contains_key(&container_id) {
                let stream = get_container_stats_stream(docker.clone(), container_id.clone()).await;
                streams.insert(container_id, stream);
            }
        }

        let mut updates = Vec::new();
        for stream in streams.values_mut() {
            let stats = stream.next().await.unwrap();
            updates.push(stats);
        }

        let still_connected = broadcast(state.clone(), updates).await;
        if !still_connected {
            info!("No more connected peers");
            return ();
        }

        // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
