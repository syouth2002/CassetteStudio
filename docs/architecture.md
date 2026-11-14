# Source responsibilities

Version 0.2.0 uses the shared Rust UI. See [build instructions](development.md). Legacy HTML sections describe compatibility code.

See [the Japanese guide](architecture.ja.md) for a complete file-by-file table.

## Current Rust UI

Native Windows/Linux/macOS windows and the browser share `ui/src/`. `app.rs` owns application state, collection mutations and frame updates. Its child modules separate responsibilities:

| File | Responsibility |
| --- | --- |
| `app_layout.rs` | Toolbar, library sidebar and collection headings |
| `app_tracks.rs` | Track rows, selection, context menus and graph requests |
| `app_dialogs.rs` | Settings and editing dialogs |
| `app_events.rs` | Async response dispatch, state updates and stale-response rejection |
| `app_playback.rs` | Playback requests, navigation and transport controls |
| `play_session.rs` | Queue, current position, gap timing and side offsets, independent of browsing |
| `playback.rs` | rodio/HTMLAudio adapters and cancellable audition state |
| `transport.rs` / `platform.rs` | HTTP and platform-specific file operations |
| `arrangement.rs` | Editable capacity/fade/side continuation calculations |
| `appearance.rs` / `settings.rs` | Shared widgets, status, formatting and Japanese/English text |
| `graph.rs` / `properties.rs` | LUFS graph state and track properties |
| `jcard*.rs` / `track_drag.rs` | J-CARD editing/view geometry and track reorder feedback |

Audio calculations remain in the backend. `audio/render.rs` and `audio/graph.rs` share `processing_filter`. Track and whole-side analysis use cache identities appropriate to each output. Manual fitting shares `arrangement::fit`; automatic selection remains in server-side `planner.rs`.

### Playback order

A single selected track starts playback at its position in the full source list. Multiple selected tracks form a queue in list order. Double-click and context-menu playback retain the source list and position too. Previous/next move between tracks, clamped at the ends; previous no longer changes behavior after three seconds. Whole-side playback seeks to offsets including leading silence and gaps, with pending seeks applied after audio preparation. Queue order and settings are captured at playback start and are unaffected by browsing another list. Stop/errors clear the session; generation checks reject stale audio responses.

The Web frontend details below describe the published v0.2.0 compatibility UI.

## Rust

- `src/lib.rs`: the shared public startup API.
- `src/server/`: lifecycle and route composition (`mod.rs`), embedded resources (`assets.rs`), HTTP operations (`api.rs`), library ownership/persistence/job coordination (`state.rs`), file identity/legacy lookup/upload cleanup (`imports.rs`), and origin checks (`security.rs`).
- `src/audio/`: a small facade (`mod.rs`), external tools (`tools.rs`), tag recovery (`metadata.rs`), media ingestion (`import.rs`), measurements (`analysis.rs`), cached EBU R128 time series (`graph.rs`), DC/gain filters and preview identity (`processing.rs`), WAV/preview generation (`render.rs`), and unit tests (`tests.rs`).
- `src/model.rs`: persisted data types and setting validation.
- `src/planner.rs`: cassette arrangement and capacity checks.
- `src/bin/desktop.rs` and `src/main.rs`: native-window and browser-engine entry points.

Preview and WAV generation continue to share the same DC/gain calculations. The refactor preserves filter order, calculations and persisted formats.

## Frontend

- `web/app.js`: startup and one-time event registration.
- `web/index.html` and `web/style.css`: structure and presentation.
- `web/js/core.js`, `state.js`, `api.js`, `dialogs.js`: formatting/DOM helpers, shared state, HTTP/persistence, and notification/prompt UI.
- `web/js/view.js`: sidebar, track lists, cassette capacity and selection rendering.
- `web/js/library.js`, `cassette.js`, `settings.js`, `export.js`: feature/data operations, fitting/arrangement, settings forms, and rendering/download caches.
- `web/js/audio/`: music playback, reference tones, loudness display calculations, and the selected-track LUFS graph (`loudness-graph.js`).
- `web/js/buttons/`: event bindings grouped by library, analysis, range editing, settings, playback and shared behavior.
- `web/i18n.js` and `web/en.json`: language selection, translation and preference persistence.

The frontend remains build-free classic JavaScript, rather than isolated ES modules. Shared variables are declared in `state.js`; HTML explicitly lists script dependencies and loads `app.js` last. Repainting and changing language do not register handlers again. New scripts must be added both to HTML and the embedded allowlist in `server/assets.rs`.

For changes, edit `view.js`/CSS for presentation, `buttons/` for controls, `cassette.js` for editable fitting, `planner.rs` for automatic selection, and `audio/analysis.rs`/`processing.rs`/`render.rs` for measurements, corrections and rendered output.

## Validation

Run the README's Rust, frontend and synthetic audio integration checks. `node scripts/test-frontend-startup.cjs` starts an isolated engine, reads its HTML, loads every served script in real order, and exercises startup/bindings/notifications/reference-tone lifecycle/auto fit/language switching using a test DOM. It does not verify browser layout or physical audio output.

Frontend regression tests load the relevant files directly instead of extracting one-line functions from source text.

Whole-side LUFS analysis in `audio/graph.rs` reuses `render.rs` to measure a temporary WAV with leading/trailing silence and gaps. It does not concatenate per-track measurements: windows and gating remain continuous across the side. Cache keys include sources, order and settings; temporary audio is removed. `test-lufs-side-graph.mjs` checks WAV agreement and caching; `test-lufs-graph-ui.cjs` checks side switching and stale-response protection.

`output.rs` owns a dedicated recording Player and countdown, stopped on drop. It shares rendered WAV caches with normal playback, but not playback state. `PlaybackPreparing` shows normal playback preparation and cancellation; generation checks reject late responses.
