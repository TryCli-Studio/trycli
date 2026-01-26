# TryCLI

**TryCLI** is an open-source platform that allows developers to host and demo Command Line Interface (CLI) tools directly in the browser.

## Tech Stack

* **Language:** Rust 🦀
* **Frontend:** Leptos (WASM)
* **Backend:** Axum
* **Runtime:** Docker

## Quick Start

### 1. Prerequisites

* Rust, Cargo, Trunk (`cargo install trunk`)
* `rustup target add wasm32-unknown-unknown`
* Docker running

### 2. Setup Container

Start a dummy container to connect to:

```bash
docker run -d -it --name trycli-sandbox ubuntu /bin/bash
```

### 3. Configure

Create a `.env` file in the root:

```env
CONTAINER_ID=<your_container_id_from_docker_ps>
```

### 4. Run Server

```bash
cd server
cargo run
```

### 5. Run Client

```bash
cd client
trunk serve --open
```
