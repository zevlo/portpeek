# portpeek

portpeek⁠ answers one simple question: is a service listening on this TCP port?

Use it for the daily cases: "did postgres come up on 5432?" or "did docker
move it to 5433?"

For each port you get one of three answers:

- `open`: the handshake completed. Something is listening.
- `closed`: the host sent RST. The host is up and the port is free.
- `filtered`: silence until the timeout. A firewall may be dropping packets.

## Install

You need Rust 1.85 or later.

```sh
cargo install --path .
```

With Docker:

```sh
docker build --provenance=false -t portpeek .
docker run --rm portpeek 127.0.0.1 22,80,443,5432,6379,8080
```

## Usage

```
portpeek [HOST] [PORT_SPEC] [OPTIONS]
```

Examples:

```sh
portpeek 127.0.0.1 80,443,5432          # check a few ports
portpeek 1-1024 --open-only             # default host, well-known ports
portpeek ::1 80,443                     # IPv6 loopback
portpeek 127.0.0.1 5432,6379 --wait 60s # poll until both ports are up
```

Write durations as `500ms`, `2s`, `1m`, or `1h`. A bare number means seconds.

| Flag | Default | Meaning |
|------|---------|---------|
| `--allow-remote` | off | Allow scanning of remote hosts |
| `--wait <DUR>` | off | Scan again every 500ms until all ports are open |
| `--concurrency <N>` | 200 | Limit concurrent connect() calls |
| `--timeout <DUR>` | 1s | Set the per-port connect timeout |
| `--open-only` | off | Show open ports only |

Exit codes:

- `0`: at least one port is open. With `--wait`: all ports opened in time.
- `1`: every port was closed or filtered. With `--wait`: the deadline passed first.
- `2`: the arguments failed validation, or the host was refused.

## Safety

portpeek scans `127.0.0.1` by default. It refuses every other host, including
RFC1918 addresses such as `192.168.1.1`, until you pass `--allow-remote`. The
check reads the host you typed. It skips DNS.

Scanning a machine without permission can be illegal in your jurisdiction.
`--allow-remote` asks you to confirm your intent before you proceed.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## License

MIT. See [LICENSE](LICENSE).
