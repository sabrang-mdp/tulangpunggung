# Tulangpunggung

**Tulangpunggung** is the backend service for **Balungpisah**, a civic platform that enables citizens of Indonesia to directly surface public issues to government institutions in a structured, transparent, and traceable way.

This repository is a **proof of concept**, not a production-ready system.

---

## Concept

Balungpisah is built around a simple principle:

> **Government reports progress. Citizens decide when a problem is solved.**

The platform allows grassroots problems to be:

* Collected from citizens
* Distilled into clearer problem statements
* Clustered into thematic or jurisdictional groups
* Presented as a “problem shop” for government bodies

Government institutions (national to district level) may:

* Select problems relevant to their mandate
* Report progress or actions taken

Citizens may:

* Monitor each problem
* Add facts or supporting information
* Review reported progress
* Decide whether a problem is resolved or still open

There is **no automatic closure by the government**.

---

## About This Codebase

This repository contains the backend service (**Tulangpunggung**) implemented in **Rust**, using **RWF**.

It currently includes:

* HTTP handlers (auth, dashboard, reports, tickets, etc.)
* WebSocket handling
* Background jobs
* Database migrations
* Basic middleware (auth, RBAC, CORS)
* Early experiments with LLM-assisted processing

The structure reflects rapid exploration and experimentation.

It is **not an endorsement of RWF as a framework choice**.

---

## ⚠️ Status

* Proof of Concept
* Not security audited
* Not production ready
* APIs are unstable
* Schema may change at any time

Use at your own risk.

---

## Project Structure (High-Level)

```
src/
├── handlers/     # HTTP endpoints
├── middleware/   # Auth, RBAC, CORS
├── background/   # Async jobs
├── websocket/    # Real-time updates
├── services/     # External / LLM services
├── db.rs         # Database layer
├── models.rs     # Data models
└── main.rs       # Entry point
```

---

## 🧪 Running Locally

Requirements:

* Rust (stable)
* PostgreSQL
* Docker (optional)

```
cp .env.example .env
cargo run
sqlx migrate run
```

---

## Philosophy

This project is intentionally:

* Citizen-first
* Skeptical of top-down “engagement” platforms
* Focused on accountability rather than sentiment

It does not attempt to:

* Replace democratic processes
* Automate political decisions
* Gamify participation

---

## License

MIT License. See `LICENSE`.

---

## Disclaimer

This software is not affiliated with, endorsed by, or connected to any government institution.

It represents an independent technical exploration of civic accountability tooling.
