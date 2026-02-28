---
name: rust-async-patterns
description: >
  Advanced Rust async patterns with Tokio, channels, cancellation, and bounded concurrency.
  Trigger: When working with async fn, tokio, channels, task orchestration, or async performance issues.
license: Apache-2.0
allowed-tools: Read, Edit, Write, Glob, Grep, Bash
metadata:
  author: "@yacosta738"
  version: "1.1"
---

# Rust Async Patterns Skill

Production-ready patterns for async Rust with Tokio: correctness, cancellation safety, and throughput.

## When to Use

- Code with `async fn`, `.await`, `tokio::`, `select!`, or streams
- Task orchestration with `JoinSet`, `spawn`, semaphores, and backpressure
- Channel-driven architectures (`mpsc`, `broadcast`, `oneshot`, `watch`)
- Graceful shutdown and cancellation propagation
- Debugging async deadlocks, starvation, or performance bottlenecks

## Principles

- Correctness before throughput: no dropped critical errors, no hidden panics
- Bounded concurrency by default
- Cancellation is a first-class behavior
- Never block async executors with sync blocking calls

## Critical Patterns

### 1. Bounded Concurrency with `JoinSet`

```rust
use anyhow::Result;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use std::sync::Arc;

async fn process_all(items: Vec<Job>, limit: usize) -> Result<Vec<Output>> {
    let sem = Arc::new(Semaphore::new(limit));
    let mut set = JoinSet::new();

    for item in items {
        let sem_cloned = Arc::clone(&sem);
        set.spawn(async move {
            let _permit = sem_cloned.acquire_owned().await?;
            process_one(item).await
        });
    }

    let mut out = Vec::new();
    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok(v)) => out.push(v),
            Ok(Err(e)) => tracing::warn!(error = %e, "job failed"),
            Err(e) => tracing::error!(error = %e, "task join failure"),
        }
    }

    Ok(out)
}
```

### 2. Structured Cancellation with `CancellationToken`

```rust
use tokio_util::sync::CancellationToken;

async fn worker(token: CancellationToken) {
    loop {
        tokio::select! {
            _ = token.cancelled() => break,
            _ = do_work_tick() => {}
        }
    }
}

async fn run() {
    let token = CancellationToken::new();
    let child = token.child_token();
    let handle = tokio::spawn(worker(child));

    // shutdown signal
    token.cancel();
    let _ = handle.await;
}
```

### 3. Channel Selection by Intent

- `mpsc`: queue work to one consumer
- `broadcast`: fan-out events to many consumers
- `oneshot`: single response handshake
- `watch`: latest-state propagation

```rust
use tokio::sync::{mpsc, oneshot};

async fn request_response(tx: mpsc::Sender<(Command, oneshot::Sender<Result<Data>>)>) -> Result<Data> {
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send((Command::Fetch, reply_tx)).await?;
    reply_rx.await?
}
```

### 4. Timeout and Retry Without Busy Loops

```rust
use tokio::time::{timeout, Duration};

async fn fetch_with_timeout() -> Result<Response, ServiceError> {
    timeout(Duration::from_secs(3), fetch_remote())
        .await
        .map_err(|_| ServiceError::Timeout)?
}
```

Retries should include backoff and max-attempt bounds.

### 5. Async Trait Boundaries

Prefer async traits for ports/adapters where IO is expected.

```rust
use async_trait::async_trait;

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, event: Event) -> Result<()>;
    async fn load(&self, aggregate_id: &str) -> Result<Vec<Event>>;
}
```

## Common Failure Modes

- Blocking call inside async task (`std::thread::sleep`, sync IO)
- Holding `Mutex`/`RwLock` guard across `.await`
- Unbounded `spawn` under load
- Ignoring cancellation paths during shutdown
- Missing `Send` requirements in spawned futures

## Async Testing Patterns

Use deterministic async tests and isolate time/network where possible.

```rust
#[tokio::test]
async fn cancels_worker_on_shutdown() {
    let token = tokio_util::sync::CancellationToken::new();
    let child = token.child_token();

    let handle = tokio::spawn(async move { worker(child).await; 42 });
    token.cancel();

    let result = handle.await.expect("task should join");
    assert_eq!(result, 42);
}
```

## Verification Commands

```bash
cargo test
cargo fmt -- --check
cargo clippy -- -D warnings
```

## Observability and Debugging

- Instrument critical async paths with `tracing` spans
- Prefer structured logs with correlation ids
- Use `tokio-console` when diagnosing scheduling/latency issues

## Related Skills

- `rust` for general Rust architecture, errors, and Cargo hygiene
- `kotlin-coroutines` for equivalent async patterns in Kotlin modules
- `tdd` for Red -> Green -> Refactor flow on async behavior changes
