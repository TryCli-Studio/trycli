# TryCli Studio

**TryCli Studio** is an open-source, full-stack platform that enables developers to host, demo, and share Command Line Interface (CLI) tools directly in the browser.

It orchestrates isolated Docker environments on-demand, providing a seamless "Repl.it-like" experience specifically optimized for terminal applications.

## Features

* **Instant Sandboxes:** Spawns a fresh, isolated Ubuntu container for every user session.
* **Snapshot & Publish:** Commits the live container state to a Docker image and generates a shareable URL.
* **Interactive Guides:** Split-pane interface with a rich Markdown editor (GitHub-flavored) and a real-time terminal.
* **Modern UI:** A "Cyberpunk/VS Code" aesthetic with glassmorphism, dark mode, and responsive layout.
* **100% Rust:** Built with a high-performance Rust stack from the kernel to the browser.

## Tech Stack

### Backend (The Orchestrator)

* **Framework:** [Axum](https://github.com/tokio-rs/axum) (Async Web Framework)
* **Runtime:** [Tokio](https://tokio.rs/)
* **Container Engine:** Docker (via [Bollard](https://github.com/fussybeaver/bollard))
* **Database:** PostgreSQL (via [SQLx](https://github.com/launchbadge/sqlx))
* **WebSocket:** Real-time STDIN/STDOUT streaming via `axum::ws`.

### Frontend (The Client)

* **Framework:** [Leptos](https://leptos.dev/) (WASM)
* **Routing:** Leptos Router
* **Terminal:** [xterm.js](https://xtermjs.org/) (via `wasm-bindgen`)
* **Markdown:** `pulldown-cmark` (Rust-based parsing)
* **Styling:** CSS Variables, Glassmorphism, Google Fonts (Inter + JetBrains Mono).

## Prerequisites

* **Rust & Cargo:** (Latest Stable)
* **Docker:** (Daemon must be running)
* **PostgreSQL:** (Running locally or via Docker)
* **WASM Target:** `rustup target add wasm32-unknown-unknown`
* **Trunk:** `cargo install trunk`

## Quick Start

### 1. Database Setup

Start a PostgreSQL container on port **5433** to avoid conflicts:

```bash
docker run --name trycli-db -e POSTGRES_PASSWORD=password -p 5433:5432 -d postgres
```

### 2. Backend Setup

Navigate to the server directory and run the API:

```bash
cd server
cargo run
```

*Server will listen on `0.0.0.0:3000`*

### 3. Frontend Setup

Navigate to the client directory and start the dev server:

```bash
cd client
trunk serve --open
```

*Browser will open at `http://localhost:8080`*

## Usage Guide

1. **Create a Demo:**
    * Go to `/new`.
    * Wait for the "Initializing Environment..." message to clear.
    * Install your CLI tool in the terminal (e.g., `apt update && apt install ...`).
    * Write instructions in the Markdown editor.
    * Enter a unique **Slug** (e.g., `my-cool-tool`).
    * Click **Publish Demo**.

2. **Share:**
    * Copy the URL (e.g., `http://localhost:8080/project/my-cool-tool`).
    * Send it to users. They will get a fresh clone of the environment you set up!

## Security Architecture

### Embed Authorization

TryCli Studio implements a dual-layer security model for embedded projects:

1. **VIP Pass (embed_key):** A private key that grants unrestricted access to embedded projects. Only project owners have access to this key.
2. **Guest List (whitelist):** A list of authorized URLs that can embed the project publicly.

#### Embed Key Protection

To prevent accidental exposure of the `embed_key` through browser dev tools or network inspection:

* The `embed_key` is **not** included in the main project response (`/api/project/:username/:slug`)
* Instead, a dedicated authenticated endpoint (`/api/project/:slug/embed-key`) is used to retrieve the key
* This endpoint requires authentication and verifies project ownership
* The key is only fetched when the user explicitly clicks the "Share / Embed" button

This separation ensures that:
* Screen sharing during project viewing won't expose the key
* Browser extensions or network logs won't capture the key during normal browsing
* The key is only retrieved when intentionally needed for sharing purposes

## Troubleshooting

* **"Container ID not found":** Ensure you wait for the terminal to initialize before clicking Publish.
* **"RowNotFound":** The project slug does not exist in the database.
* **Database Connection Refused:** Ensure your Docker container is running on port **5433**, or update the connection string in `server/src/main.rs`.
