use std::time::{Duration, Instant};

use tokio::time::sleep;

use crate::scan::{self, PortResult, Status};

const RETRY_INTERVAL: Duration = Duration::from_millis(500);

pub struct WaitOutcome {
    pub results: Vec<PortResult>,
    pub all_open: bool,
    pub attempts: u32,
}

pub async fn wait_for_all(
    host: &str,
    ports: &[u16],
    concurrency: usize,
    per_port_timeout: Duration,
    deadline: Duration,
) -> WaitOutcome {
    let started = Instant::now();
    let mut attempts = 0u32;
    loop {
        attempts += 1;
        let results = scan::scan(host, ports, concurrency, per_port_timeout).await;
        let open = results.iter().filter(|r| r.status == Status::Open).count();
        eprintln!("attempt {attempts}: {open}/{} open", ports.len());
        if open == ports.len() {
            return WaitOutcome {
                results,
                all_open: true,
                attempts,
            };
        }
        let elapsed = started.elapsed();
        if elapsed >= deadline {
            return WaitOutcome {
                results,
                all_open: false,
                attempts,
            };
        }
        sleep((deadline - elapsed).min(RETRY_INTERVAL)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn returns_when_all_open() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let outcome = wait_for_all(
            "127.0.0.1",
            &[port],
            10,
            Duration::from_millis(500),
            Duration::from_secs(5),
        )
        .await;
        assert!(outcome.all_open);
        assert_eq!(outcome.attempts, 1);
    }

    #[tokio::test]
    async fn gives_up_after_deadline() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let started = Instant::now();
        let outcome = wait_for_all(
            "127.0.0.1",
            &[port],
            10,
            Duration::from_millis(500),
            Duration::from_millis(300),
        )
        .await;
        assert!(!outcome.all_open);
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[tokio::test]
    async fn succeeds_when_port_appears_late() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let later = tokio::spawn(async move {
            sleep(Duration::from_millis(700)).await;
            TcpListener::bind(("127.0.0.1", port)).await
        });
        let outcome = wait_for_all(
            "127.0.0.1",
            &[port],
            10,
            Duration::from_millis(500),
            Duration::from_secs(10),
        )
        .await;
        assert!(outcome.all_open, "should detect late bind");
        assert!(later.is_finished());
    }
}
