# portpeek

A polite, localhost-first TCP port peeker. It answers one question: for these
ports, is something listening?

`portpeek` is for the fifteen-times-a-day case: "did postgres come up on 5432,
or did docker bump it to 5433 again?" It is not nmap. There is no SYN scan,
no OS fingerprinting, no service probing. Just concurrent TCP connects with
an honest three-way answer per port:

- `open` — the handshake completed, something is listening
- `closed` — the host answered with RST, nothing is listening
- `filtered` — silence until the timeout; a firewall may be dropping packets

## Install

Requires Rust 1.85 or later.

```sh
cargo install --path .
```

Or with Docker:

```sh
docker build -t portpeek .
docker run --rm portpeek 127.0.0.1 22,80,443,5432,6379,8080
```

## Usage

```sh
portpeek [HOST] [PORT_SPEC] [OPTIONS]
```

Examples:

```sh
portpeek 127.0.0.1 80,443,5432          # scan a few ports
portpeek 1-1024 --open-only             # default host, classic well-knowns
portpeek ::1 80,443                     # IPv6 loopback
portpeek 127.0.0.1 5432,6379 --wait 60s # poll until both are up
```

Options:

| Flag | Default | Meaning |
|------|---------|---------|
| `--allow-remote` | off | Permit scanning non-loopback hosts |
| `--wait <DUR>` | off | Re-scan every 500ms until all ports are open |
| `--concurrency <N>` | 200 | Max concurrent connect() calls |
| `--timeout <DUR>` | 1s | Per-port connect timeout |
| `--open-only` | off | Hide closed and filtered rows |

Durations accept `500ms`, `2s`, `1m`, `1h`; a bare number means seconds.

Exit codes:

- `0` — at least one port open (or all ports open by the `--wait` deadline)
- `1` — no port open, or the `--wait` deadline expired
- `2` — bad arguments or remote-host refusal

## Safety

`portpeek` scans `127.0.0.1` by default. Any non-loopback target, including
RFC1918 addresses like `192.168.1.1`, is refused with exit code 2 unless you
pass `--allow-remote`. Classification is syntactic; DNS is never resolved
before the gate decides.

Scanning a machine you do not own can be rude at best and illegal at worst,
depending on your jurisdiction. `--allow-remote` is a nudge, not a security
feature: if you own the target or have permission, type the flag and proceed.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## License

MIT. See [LICENSE](LICENSE).
