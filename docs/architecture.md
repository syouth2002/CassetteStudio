# Source responsibilities

See [the Japanese guide](architecture.ja.md) for a complete file-by-file table.

## Rust

- `src/lib.rs`: the shared public startup API.
- `src/server/`: lifecycle and route composition (`mod.rs`), embedded resources (`assets.rs`), HTTP operations (`api.rs`), library ownership/persistence/job coordination (`state.rs`), and origin checks (`security.rs`).
- `src/audio/`: a small facade (`mod.rs`), external tools (`tools.rs`), tag recovery (`metadata.rs`), media ingestion (`import.rs`), measurements (`analysis.rs`), DC/gain filters and preview identity (`processing.rs`), WAV/preview generation (`render.rs`), and unit tests (`tests.rs`).
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
- `web/js/audio/`: music playback, reference tones and loudness display calculations.
- `web/js/buttons/`: event bindings grouped by library, analysis, range editing, settings, playback and shared behavior.
- `web/i18n.js` and `web/en.json`: language selection, translation and preference persistence.

The frontend remains build-free classic JavaScript, rather than isolated ES modules. Shared variables are declared in `state.js`; HTML explicitly lists script dependencies and loads `app.js` last. Repainting and changing language do not register handlers again. New scripts must be added both to HTML and the embedded allowlist in `server/assets.rs`.

For changes, edit `view.js`/CSS for presentation, `buttons/` for controls, `cassette.js` for editable fitting, `planner.rs` for automatic selection, and `audio/analysis.rs`/`processing.rs`/`render.rs` for measurements, corrections and rendered output.

## Validation

Run the README's Rust, frontend and synthetic audio integration checks. `node scripts/test-frontend-startup.cjs` starts an isolated engine, reads its HTML, loads every served script in real order, and exercises startup/bindings/notifications/reference-tone lifecycle/auto fit/language switching using a test DOM. It does not verify browser layout or physical audio output.

Frontend regression tests load the relevant files directly instead of extracting one-line functions from source text.
