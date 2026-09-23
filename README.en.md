<div align="center">

# Canghai · API Debugger

**Lightweight cross-platform HTTP debugging desktop app built with Tauri 2 + Vue 3**

No CORS · Multi-tab · Environment Groups · Pre/Post Scripts · Mock Responses · Import/Export · Workspaces · Cloud Sync

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Vue](https://img.shields.io/badge/Vue-3.5-42b883.svg)](https://vuejs.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-ffc131.svg)](https://tauri.app/)
[![Element Plus](https://img.shields.io/badge/Element%20Plus-2.9-409eff.svg)](https://element-plus.org/)
[![Spring Boot](https://img.shields.io/badge/Spring%20Boot-4.x-6db33f.svg)](https://spring.io/projects/spring-boot)

[中文](README.md)

</div>

---

## Overview

Canghai API Debugger is a lightweight desktop HTTP client for developers. By routing all requests through a native Tauri (Rust) backend, it completely eliminates browser CORS restrictions — making it just as easy to debug local services, intranet APIs, and production endpoints.

The server is built on **Spring Boot 4** (webmvc + MyBatis-Plus + Druid + Fastjson2), integrating JWT (RS256) authentication, providing workspace collaboration and cloud data sync.

---

## Features

### Core Debugging
- **Multi-tab workspace**: Open multiple requests simultaneously, switch freely, each tab is fully independent; double-click a tab to rename it, right-click for context menu (close others/close all)
- **All HTTP methods**: GET / POST / PUT / PATCH / DELETE / HEAD / OPTIONS
- **Request body types**: JSON (with comment support), form-urlencoded, plain text, none
- **Params & Headers editor**: Visual key-value editor with per-row enable/disable toggle
- **Response viewer**: Pretty / Raw views, header table, request details, status code, time elapsed, and body size at a glance
- **Keyboard shortcuts**: Press `Ctrl+S` to quickly save the current request

### Environments
- Create multiple environments (development / staging / production) and switch with one click
- **Environment groups**: Organize environments by project or purpose with group management
- Reference variables anywhere using `{{variableName}}` syntax — in URL, headers, body, and scripts
- Variables can be written at runtime via scripts using `env.set()`
- Script-written variables are immediately available for `{{var}}` resolution in the current request

### Pre & Post Request Scripts
- Execute custom JavaScript before a request is sent or after a response is received (async/await supported)
- Pre-scripts can dynamically modify `req.url`, `req.headers`, and `req.body`
- Post-scripts can read `res.status`, `res.body`, `res.json()` and persist values to environment variables
- **Built-in CryptoJS**: Supports MD5, SHA256, HMAC, AES, Base64 and other cryptographic algorithms
- Built-in `console`, `test()`, and `assert()` helpers; script output appears in a dedicated log tab

### Request Management
- Save requests in a nested category tree for quick access
- **Drag & drop categories**: Drag category nodes to reorganize the tree structure
- **Import / Export**: Export all saved requests as a JSON file for backup, or batch import from a JSON file
- Save new requests or overwrite existing ones with a single click
- Automatic history tracks every request for fast lookback

### Workspaces & Cloud Sync
- **Multi-workspace**: Create independent workspaces to isolate API data for different projects or teams
- **Cloud sync**: Log in and sync environments, requests, and categories to the server
- **Role-based access**: Owner, Admin, Member, and Read-only roles; read-only members cannot modify data
- **Server**: Spring Boot server for data persistence and team collaboration

### Three Work Modes

| Mode | Description |
|------|-------------|
| **Debug** | Primary mode — send requests and inspect responses in real time |
| **Preview** | View the resolved URL, params, and headers; generate a cURL command with one click |
| **Design** | Write Markdown API documentation and configure mock scenarios for frontend development without a real backend |

### User Experience
- **Resizable sidebar**: Drag the sidebar edge to adjust width, with local persistence
- **Smart tab reuse**: Clicking an already-open request reuses the existing tab, avoiding duplicates
- **Breadcrumb navigation**: Shows the current request's category path and edit status

### Zero CORS Restrictions
All HTTP requests are issued by the Tauri Rust backend, completely bypassing the browser's same-origin policy.

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | [Tauri 2](https://tauri.app/) |
| Frontend | [Vue 3.5](https://vuejs.org/) + [TypeScript 5.7](https://www.typescriptlang.org/) |
| UI library | [Element Plus 2.9](https://element-plus.org/) |
| Build tool | [Vite 6](https://vitejs.dev/) |
| Markdown | [marked 18](https://marked.js.org/) |
| Crypto library | [crypto-js 4.2](https://github.com/brix/crypto-js) |
| HTTP client | [reqwest 0.12](https://github.com/seanmonstar/reqwest) (rustls-tls) |
| Database | [rusqlite 0.31](https://github.com/rusqlite/rusqlite) (SQLite) |
| Package manager | [pnpm](https://pnpm.io/) (monorepo workspace) |
| Server framework | [Spring Boot 4](https://spring.io/projects/spring-boot) (webmvc + MyBatis-Plus + Druid + Fastjson2) |
| Server database | MySQL |
| JSON library | [Fastjson2](https://github.com/alibaba/fastjson2) |
| Connection pool | [Druid](https://github.com/alibaba/druid) |

---

## Project Structure

```
canghai-api-doc/
├── client/
│   ├── apps/
│   │   ├── web/                        # Vue 3 frontend
│   │   │   └── src/
│   │   │       ├── components/         # Reusable components
│   │   │       │   ├── CategoryTree.vue        # Category sidebar (with import/export)
│   │   │       │   ├── CategoryNode.vue        # Category node (drag & drop support)
│   │   │       │   ├── KeyValueEditor.vue      # Key-value row editor
│   │   │       │   ├── EnvironmentManager.vue  # Environment manager dialog (with groups)
│   │   │       │   └── TeamManager.vue          # Team manager dialog
│   │   │       ├── composables/        # Vue 3 composables (state & logic)
│   │   │       │   ├── useHttpRequest.ts       # HTTP client (Tauri invoke)
│   │   │       │   ├── useScriptEngine.ts      # Pre/post script execution (with CryptoJS)
│   │   │       │   ├── useEnvironments.ts      # Environment variables & groups
│   │   │       │   ├── useTabs.ts              # Multi-tab management (with context menu)
│   │   │       │   ├── useHistory.ts           # Request history
│   │   │       │   ├── useSavedRequests.ts     # Saved request CRUD (import/export)
│   │   │       │   ├── useCategories.ts        # Category CRUD (with drag & drop)
│   │   │       │   ├── useSettings.ts          # Global settings
│   │   │       │   ├── useSync.ts              # Cloud sync & user authentication
│   │   │       │   ├── useSidebar.ts           # Sidebar width adjustment
│   │   │       │   └── useTeams.ts              # Team management
│   │   │       ├── views/
│   │   │       │   └── ApiDebuggerView.vue     # Main view
│   │   │       └── types/index.ts              # Global TypeScript types
│   │   └── desktop/                    # Tauri desktop app
│   │       └── src/                    # Rust backend
│   │           ├── commands/           # Tauri command modules
│   │           │   ├── http.rs         # HTTP request proxy
│   │           │   ├── category.rs     # Category CRUD
│   │           │   ├── saved_request.rs # Saved request CRUD
│   │           │   ├── history.rs      # History CRUD
│   │           │   ├── environment.rs  # Environment variables & groups CRUD
│   │           │   ├── sync.rs         # Data sync
│   │           │   └── workspace.rs    # Workspace CRUD
│   │           ├── db.rs               # SQLite database initialization
│   │           ├── models.rs           # Data model definitions
│   │           ├── lib.rs              # Library entry
│   │           └── main.rs             # Application entry
│   ├── package.json
│   └── pnpm-workspace.yaml

```

---

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/) 8+
- [Rust](https://www.rust-lang.org/) 1.77+ (required for the Tauri desktop build)
- [Tauri system dependencies](https://tauri.app/start/prerequisites/) (varies by platform — see official docs)

### Install dependencies

```bash
cd client
pnpm install
```

### Development

```bash
# Start the web frontend only (browser preview — CORS restrictions apply)
pnpm web:dev

# Start the full desktop app (recommended — no CORS restrictions)
pnpm dev
```

### Production build

```bash
pnpm build
```

Build artifacts are output to `client/apps/desktop/target/release/bundle/`.

### Start the server (optional)

The server enables cloud sync and team collaboration — it is not required for standalone use.

```bash
cd server
mvn package -DskipTests
java -jar target/canghai-server-0.1.1.jar
```

The server listens on `http://localhost:8092` by default.

---

## Usage Guide

### Sending your first request

1. Enter the endpoint URL and select an HTTP method
2. Fill in parameters under the **Params**, **Headers**, or **Body** tabs
3. Click **Send** — the response appears in the panel below
4. Use `Ctrl+S` to quickly save the current request

### Using environment variables

1. Click the edit icon next to the environment selector in the top bar to open the Environment Manager
2. Optionally create environment groups (e.g., "Project A", "Project B"), then create environments within groups
3. Create an environment and add variables (e.g. `baseUrl = https://api.example.com`)
4. Reference them in your URL: `{{baseUrl}}/users`
5. When you switch environments, all references update automatically

### Writing scripts

**Pre-request script** — inject an auth token and calculate signature:
```javascript
const token = env.get('token')
req.headers['Authorization'] = 'Bearer ' + token

// Use CryptoJS to calculate request signature
const sign = CryptoJS.MD5(req.body + env.get('secret')).toString()
req.headers['X-Sign'] = sign

// Set a timestamp variable, can be referenced via {{timestamp}} in current request
env.set('timestamp', Date.now().toString())
req.params['ts'] = '{{timestamp}}'
```

**Post-request script** — extract and store a token from the response:
```javascript
const data = res.json()
if (data.token) {
  env.set('token', data.token)
  console.log('Token saved')
}
test('Status is 200', () => assert(res.status === 200))
```

### Mock responses (Design mode)

1. Switch to **Design** mode
2. Write your API documentation in the Markdown editor
3. Click **+ Add Mock Scenario** and configure the status code, response headers, and body
4. Click **Mock Send** to simulate a response without a real backend

### Managing the category tree

- **Drag & drop**: Drag a category node onto a target category to move it
- **Context menu**: Click the `···` button on a category node to add an API, add a subcategory, edit, or delete

### Import / Export APIs

- Click the **More actions** (`···`) button next to the category tree title, select **Export** to save all requests as a JSON file
- Select **Import** to batch import requests from a JSON file; duplicate names are automatically skipped

### Workspaces & sync

- Click the workspace button in the top toolbar to open the workspace manager — create and switch between multiple workspaces
- After logging in, click the **Sync** button to push local data to the server or pull remote updates

---

## Contributing

1. Fork the repository
2. Create a branch: `feat/your-feature`
3. Commit your changes and verify `pnpm build` passes
4. Open a Pull Request

---

---
## Acknowledgements

Thanks to the following open source projects that power this tool:

- [Tauri](https://tauri.app/) — Cross-platform desktop framework powered by Rust
- [Vue 3](https://vuejs.org/) — The Progressive JavaScript Framework
- [Element Plus](https://element-plus.org/) — Vue 3 UI component library
- [Vite](https://vitejs.dev/) — Next generation frontend build tool
- [marked](https://marked.js.org/) — Markdown parser and compiler
- [crypto-js](https://github.com/brix/crypto-js) — JavaScript cryptography library
- [reqwest](https://github.com/seanmonstar/reqwest) — Rust HTTP client
- [rusqlite](https://github.com/rusqlite/rusqlite) — Rust bindings for SQLite
- [Spring Boot](https://spring.io/projects/spring-boot) — Backend service framework
- [Undertow](https://undertow.io/) — High performance Java web server
- [Fastjson2](https://github.com/alibaba/fastjson2) — High performance JSON library
- [Druid](https://github.com/alibaba/druid) — Database connection pool

---
## License

[MIT](LICENSE)
