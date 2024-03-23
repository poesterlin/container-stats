use bollard::container::{ListContainersOptions, Stats, StatsOptions};
use bollard::Docker;
use futures_util::stream::StreamExt;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ContainerStats {
    pub id: String,
    pub name: String,
    pub memory_usage: String,
    pub cpu_usage: f64,
}

impl From<Stats> for ContainerStats {
    fn from(stats: Stats) -> Self {
        ContainerStats {
            id: stats.id.clone(),
            name: stats.name.clone().split_off(1),
            memory_usage: human_readable_bytes(stats.memory_stats.usage.unwrap()),
            cpu_usage: calculate_cpu_percent(stats),
        }
    }
}

pub async fn collect_all_stats(docker: &Docker) -> Vec<ContainerStats> {
    let mut stats: Vec<ContainerStats> = Vec::new();

    let mut filter = HashMap::new();
    filter.insert(String::from("status"), vec![String::from("running")]);

    let options = Some(ListContainersOptions {
        all: true,
        filters: filter,
        ..Default::default()
    });

    let containers = docker.clone().list_containers(options).await.unwrap();

    let mut handles = Vec::new();
    for container in containers {
        let container_id = container.id.to_owned().unwrap();
        let d = docker.clone();
        let job = tokio::spawn(get_container_stats(d, container_id));
        handles.push(job);
    }

    println!("Waiting for all jobs to complete");

    for job in handles {
        match job.await.unwrap() {
            Some(stat) => stats.push(stat.into()),
            None => (),
        }
    }

    stats
}

async fn get_container_stats(docker: Docker, container_id: String) -> Option<Stats> {
    println!("Getting stats for container: {}", container_id);

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

fn calculate_cpu_percent(stats: Stats) -> f64 {
    let cpu_delta =
        stats.cpu_stats.cpu_usage.total_usage - stats.precpu_stats.cpu_usage.total_usage;
    let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0)
        - stats.precpu_stats.system_cpu_usage.unwrap_or(0);

    let cpu_percent = (cpu_delta as f64 / system_delta as f64) * 100.0;

    cpu_percent
}
