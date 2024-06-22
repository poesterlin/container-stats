use bollard::container::{ListContainersOptions, Stats, StatsOptions};
use bollard::Docker;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContainerStats {
    pub id: String,
    pub name: String,
    pub memory_usage: String,
    pub cpu_usage: String,
}

impl From<Stats> for ContainerStats {
    fn from(stats: Stats) -> Self {
        let id_prefix = "container_";
        let short_id = stats.id.clone().split_off(64 - 8);
        let id = format!("{}{}", id_prefix, short_id);
        ContainerStats {
            id,
            name: stats.name.clone().split_off(1),
            memory_usage: human_readable_bytes(stats.memory_stats.usage.unwrap_or(0)),
            cpu_usage: calculate_cpu_percent(stats),
        }
    }
}

pub async fn list_running_containers(docker: &Docker) -> Vec<String> {
    let mut filter = HashMap::new();
    filter.insert(String::from("status"), vec![String::from("running")]);

    let options = Some(ListContainersOptions {
        all: true,
        filters: filter,
        ..Default::default()
    });

    let containers = docker.clone().list_containers(options).await.unwrap();

    containers
        .iter()
        .map(|container| container.id.clone().unwrap())
        .collect()
}

pub async fn collect_all_stats(docker: &Docker) -> Vec<ContainerStats> {
    let mut stats: Vec<ContainerStats> = Vec::new();

    let containers = list_running_containers(docker).await;

    let mut handles = Vec::new();
    for container_id in containers {
        let d = docker.clone();
        let job = tokio::spawn(get_container_stats(d, container_id));
        handles.push(job);
    }

    for job in handles {
        match job.await.unwrap() {
            Some(stat) => stats.push(stat.into()),
            None => (),
        }
    }

    stats
}

pub async fn get_container_stats(docker: Docker, container_id: String) -> Option<Stats> {
    let status = docker
        .stats(
            container_id.as_str(),
            Some(StatsOptions {
                stream: false,
                ..Default::default()
            }),
        )
        .next()
        .await
        .unwrap();

    match status {
        Ok(stats) => Some(stats),
        Err(_) => None,
    }
}

fn human_readable_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "Error".to_string();
    }

    let kb = bytes / 1024;
    let mb = kb / 1024;
    let gb = mb / 1024;

    if gb > 0 {
        format!("{} GB", gb)
    } else if mb > 0 {
        format!("{} MB", mb)
    } else if kb > 0 {
        format!("{} KB", kb)
    } else {
        format!("{} B", bytes)
    }
}

fn calculate_cpu_percent(stats: Stats) -> String {
    let cpu_delta =
        stats.cpu_stats.cpu_usage.total_usage - stats.precpu_stats.cpu_usage.total_usage;
    let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0)
        - stats.precpu_stats.system_cpu_usage.unwrap_or(0);

    let cpu_percent = (cpu_delta as f64 / system_delta as f64) * 100.0;

    format!("{:.2}%", cpu_percent)
}
