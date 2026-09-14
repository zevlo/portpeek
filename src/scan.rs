use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Open,
    Closed,
    Filtered,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::Closed => "closed",
            Status::Filtered => "filtered",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PortResult {
    pub port: u16,
    pub status: Status,
    pub latency: Duration,
}

pub async fn scan(
    host: &str,
    ports: &[u16],
    concurrency: usize,
    per_port_timeout: Duration,
) -> Vec<PortResult> {
    let sem = Arc::new(Semaphore::new(concurrency.max(1)));
    let host: Arc<str> = Arc::from(host);
    let mut set: JoinSet<PortResult> = JoinSet::new();

    for &port in ports {
        let sem = sem.clone();
        let host = host.clone();
        set.spawn(async move {
            let _permit = sem.acquire_owned().await.expect("semaphore closed");
            probe_one(&host, port, per_port_timeout).await
        });
    }

    let mut results = Vec::with_capacity(ports.len());
    while let Some(joined) = set.join_next().await {
        if let Ok(r) = joined {
            results.push(r);
        }
    }
    results.sort_by_key(|r| r.port);
    results
}

async fn probe_one(host: &str, port: u16, deadline: Duration) -> PortResult {
    let started = Instant::now();
    let result = timeout(deadline, TcpStream::connect((host, port))).await;
    let latency = started.elapsed();
    let status = match result {
        Ok(Ok(_stream)) => Status::Open,
        Ok(Err(_)) => Status::Closed,
        Err(_) => Status::Filtered,
    };
    PortResult {
        port,
        status,
        latency,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    async fn bound_listener(host: &str) -> (TcpListener, u16) {
        let listener = TcpListener::bind((host, 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        (listener, port)
    }

    #[tokio::test]
    async fn open_port_is_detected() {
        let (_listener, port) = bound_listener("127.0.0.1").await;
        let results = scan("127.0.0.1", &[port], 10, Duration::from_millis(500)).await;
        assert_eq!(results[0].status, Status::Open);
    }

    #[tokio::test]
    async fn closed_port_is_detected() {
        let (listener, port) = bound_listener("127.0.0.1").await;
        drop(listener);
        let results = scan("127.0.0.1", &[port], 10, Duration::from_millis(500)).await;
        assert_eq!(results[0].status, Status::Closed);
    }

    #[tokio::test]
    async fn ipv6_loopback_open_is_detected() {
        let (_listener, port) = bound_listener("::1").await;
        let results = scan("::1", &[port], 10, Duration::from_millis(500)).await;
        assert_eq!(results[0].status, Status::Open);
    }

    #[tokio::test]
    async fn results_are_sorted_by_port() {
        let (_a, pa) = bound_listener("127.0.0.1").await;
        let (_b, pb) = bound_listener("127.0.0.1").await;
        let hi = pa.max(pb);
        let lo = pa.min(pb);
        let results = scan("127.0.0.1", &[hi, lo], 10, Duration::from_millis(500)).await;
        let ports: Vec<u16> = results.iter().map(|r| r.port).collect();
        assert_eq!(ports, vec![lo, hi]);
    }

    #[tokio::test]
    async fn mixed_statuses_reported_per_port() {
        let (_open_l, open_port) = bound_listener("127.0.0.1").await;
        let (closed_l, closed_port) = bound_listener("127.0.0.1").await;
        drop(closed_l);
        let results = scan(
            "127.0.0.1",
            &[open_port, closed_port],
            10,
            Duration::from_millis(500),
        )
        .await;
        for r in &results {
            match r.port {
                p if p == open_port => assert_eq!(r.status, Status::Open),
                p if p == closed_port => assert_eq!(r.status, Status::Closed),
                _ => unreachable!(),
            }
        }
    }

    #[tokio::test]
    async fn unroutable_target_is_not_open() {
        let results = scan("192.0.2.1", &[80], 1, Duration::from_millis(100)).await;
        assert_ne!(results[0].status, Status::Open);
    }
}
