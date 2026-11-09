<a id="readme-top"></a>

<div align="center">
  <h1>Orvix</h1>

  <p>
    A self-hosted video conferencing platform built on a custom Rust SFU and a Vue 3 client.
    <br />
    Own your meetings, your media, and your data.
  </p>

  <p>
    <a href="docs/md/index.md"><strong>Read the Docs</strong></a>
    &middot;
    <a href="https://github.com/marfanr/vidcall/issues/new?labels=bug"><strong>Report Bug</strong></a>
    &middot;
    <a href="https://github.com/marfanr/vidcall/issues/new?labels=enhancement"><strong>Request Feature</strong></a>
  </p>

  <img src="https://img.shields.io/github/license/marfanr/vidcall.svg?style=for-the-badge" alt="License: AGPL-3.0" />
  <img src="https://img.shields.io/github/stars/marfanr/vidcall.svg?style=for-the-badge" alt="Stars" />
  <img src="https://img.shields.io/github/issues/marfanr/vidcall.svg?style=for-the-badge" alt="Issues" />

  <br /><br />

  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/Axum-0F172A?style=for-the-badge&logo=rust&logoColor=white" alt="Axum" />
  <img src="https://img.shields.io/badge/WebRTC-333333?style=for-the-badge&logo=webrtc&logoColor=white" alt="WebRTC" />
  <img src="https://img.shields.io/badge/PostgreSQL-4169E1?style=for-the-badge&logo=postgresql&logoColor=white" alt="PostgreSQL" />
  <img src="https://img.shields.io/badge/Vue.js_3-4FC08D?style=for-the-badge&logo=vuedotjs&logoColor=white" alt="Vue.js" />
  <img src="https://img.shields.io/badge/TypeScript-3178C6?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript" />
  <img src="https://img.shields.io/badge/Docker-2496ED?style=for-the-badge&logo=docker&logoColor=white" alt="Docker" />
  <img src="https://img.shields.io/badge/OpenTelemetry-425CC7?style=for-the-badge&logo=opentelemetry&logoColor=white" alt="OpenTelemetry" />
</div>

<br />

<div align="center">
  <img src=".assets/ss.png" alt="Orvix in-call interface" width="100%" />
</div>

## About

**Orvix** (repository: **VidCall**) is a self-hosted video conferencing system for teams that want full control over their meetings. A Rust backend with a purpose-built Selective Forwarding Unit (SFU) handles WebRTC media, authentication, and rooms, while a Vue 3 frontend provides the meeting experience. Media flows through your own infrastructure, with no third-party service involved.

## Features

- **Video & audio:** multi-party calls through a custom sans-io Rust SFU with simulcast-aware forwarding.
- **Meeting rooms:** aliases, optional passcodes, custom titles, and a waiting room with join requests.
- **In-call tools:** chat, emoji reactions, screen sharing, member list, and per-device media configuration.
- **Authentication:** JWT access tokens with rotating refresh tokens and reuse detection.
- **Recording:** server-side recording to disk (requires `ffmpeg`).
- **AI experiments:** local ONNX text-to-speech, speech-to-text, and background filters.
- **Observability:** OpenTelemetry, Prometheus, Loki, Tempo, and provisioned Grafana dashboards.

## Architecture

```mermaid
flowchart LR
    subgraph Browser["Browser · Vue 3 client"]
        UI["UI · Pinia · Router"]
        PUB["Publisher PC<br/>mic · cam · screen"]
        SUB["Subscriber PC<br/>remote tracks · chat/control"]
    end

    subgraph Backend["Rust backend · Axum"]
        API["REST API"]
        WS["WebSocket signaling"]
        subgraph Pool["Peer worker pool · sans-io<br/>one thread + runtime per core"]
            MUX["UDP mux<br/>single socket per worker<br/>ICE ufrag → peer routing"]
            DIR["PeerDriver event loop<br/>publisher + subscriber `RTCPeerConnection`<br/>handle_read · poll_write · poll_timeout"]
            ROOM["RoomSfu<br/>track fan-out · simulcast · record hooks"]
        end
    end

    DB[("PostgreSQL")]
    REC[("Recordings · ffmpeg")]
    OTEL["OpenTelemetry Collector"]
    OBS["Tempo · Loki · Prometheus · Grafana"]

    UI -->|"HTTPS · JSON"| API
    UI <-->|"WSS · signaling"| WS
    PUB <-->|"UDP · STUN/DTLS/SRTP (muxed)"| MUX
    SUB <-->|"UDP · STUN/DTLS/SRTP (muxed)"| MUX
    WS -->|"PeerCommand"| DIR
    MUX -->|"inbound packets"| DIR
    DIR <-->|"fan-out"| ROOM
    API --> DB
    ROOM --> DB
    API --> OTEL
    Pool --> OTEL
    ROOM --> REC
    OTEL --> OBS
```

For SFU internals, API reference, data models, and the full environment variable list, see the [documentation](docs/md/index.md).

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Getting Started

### Prerequisites

- Rust 1.85+
- Bun 1.2+ (or Node.js 22.18+ / 24.12+)
- Docker and Docker Compose
- OpenSSL
- FFmpeg (optional, for recording)

### Installation

1. Clone the repository:

   ```sh
   git clone git@github.com:marfanr/vidcall.git
   cd vidcall
   ```

2. Generate TLS certificates and the base environment file:

   ```sh
   cd backend
   ./setup_certs.sh
   ```

3. Add the JWT secrets to `backend/.env` (each at least 32 bytes, and different from each other):

   ```sh
   openssl rand -base64 48   # JWT_ACCESS_SECRET
   openssl rand -base64 48   # JWT_REFRESH_SECRET
   ```

   Also set `POSTGRES_PASSWORD`. All other variables have defaults; see the [docs](docs/md/index.md) for the full list.

4. Start PostgreSQL (and optionally the observability stack):

   ```sh
   docker compose up -d etcd postgres

   # Optional
   docker compose up -d otel-collector tempo loki prometheus grafana
   ```

5. Create a `vidcall` database and apply [`backend/migrations/01-init.sql`](backend/migrations/01-init.sql).

   > Migration files are still being consolidated. Verify the tables referenced in [`backend/src/models/constant.rs`](backend/src/models/constant.rs) before starting the backend.

### Run

```sh
# backend (http://localhost:8080)
cd backend && cargo run

# frontend (http://localhost:5173)
cd frontend && bun install && bun dev
```

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Roadmap

- [x] Multi-party WebRTC SFU
- [x] JWT authentication with rotating refresh tokens
- [x] Waiting room and join requests
- [x] Chat, reactions, and screen sharing
- [x] Server-side recording
- [ ] Consolidated, automated database migrations
- [ ] End-to-end encrypted media
- [ ] Horizontal SFU scaling for large rooms
- [ ] Production deployment guides and Helm charts

See the [open issues](https://github.com/marfanr/vidcall/issues) for proposed features and known issues.

## Contributing

Contributions are welcome. Fork the project, create a feature branch, and open a pull request. Before submitting, run:

```sh
# frontend
cd frontend && bun run type-check && bun run lint

# backend
cd backend && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

## License

Distributed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**. See [`LICENSE`](LICENSE) for details.

## Acknowledgments

[webrtc-rs/rtc](https://github.com/webrtc-rs/rtc), [Axum](https://github.com/tokio-rs/axum) and Tokio, [Vue.js](https://vuejs.org/), [OpenTelemetry](https://opentelemetry.io/), [Grafana](https://grafana.com/), [ONNX Runtime](https://onnxruntime.ai/), [MediaPipe](https://developers.google.com/mediapipe), and [Best-README-Template](https://github.com/othneildrew/Best-README-Template).

<p align="right">(<a href="#readme-top">back to top</a>)</p>