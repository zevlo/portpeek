use std::net::TcpListener;
use std::process::Command;
use std::time::Duration;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_portpeek"))
}

fn dropped_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn held_port() -> (TcpListener, u16) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

#[test]
fn exit_0_when_any_port_open() {
    let (_hold, port) = held_port();
    let out = bin()
        .args(["127.0.0.1", &port.to_string()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("open"), "stdout: {stdout}");
    assert!(
        stdout.contains("SERVICE"),
        "service column header missing: {stdout}"
    );
}

#[test]
fn exit_1_when_no_port_open() {
    let port = dropped_port();
    let out = bin()
        .args(["127.0.0.1", &port.to_string()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn exit_2_remote_without_allow_remote() {
    let out = bin().args(["192.168.1.1", "80"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--allow-remote"), "stderr: {stderr}");
}

#[test]
fn exit_2_on_bad_port_spec() {
    let out = bin().args(["127.0.0.1", "http"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn exit_2_on_port_zero() {
    let out = bin().args(["127.0.0.1", "0"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn gate_accepts_all_of_127_slash_8() {
    let port = dropped_port();
    let out = bin()
        .args(["127.0.0.2", &port.to_string()])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "gate passed (exit 1 = scanned, not 2 = blocked)"
    );
}

#[test]
fn ipv6_loopback_scan_works() {
    let listener = TcpListener::bind(("::1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let out = bin().args(["::1", &port.to_string()]).output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn open_only_hides_closed_rows() {
    let (_hold, open_port) = held_port();
    let closed_port = {
        let p = dropped_port();
        if p < open_port {
            return;
        }
        p
    };
    let spec = format!("{closed_port},{open_port}");
    let out = bin()
        .args(["127.0.0.1", &spec, "--open-only"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if line.contains("closed") && !line.contains(" filtered in ") {
            panic!("closed row leaked into --open-only output: {line}");
        }
    }
}

#[test]
fn wait_expires_with_exit_1() {
    let port = dropped_port();
    let started = std::time::Instant::now();
    let out = bin()
        .args(["127.0.0.1", &port.to_string(), "--wait", "300ms"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn wait_succeeds_when_port_appears() {
    let port = dropped_port();
    let child = bin()
        .args(["127.0.0.1", &port.to_string(), "--wait", "10s"])
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(700));
    let _hold = TcpListener::bind(("127.0.0.1", port)).expect("re-bind failed (TOCTOU)");
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn no_color_disables_ansi() {
    let (_hold, port) = held_port();
    let out = bin()
        .env("NO_COLOR", "1")
        .args(["127.0.0.1", &port.to_string()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains('\x1b'), "stdout contains ANSI: {stdout:?}");
}
