# Repository Guidelines

## Project Structure & Module Organization
- `src/` holds the Vue 3 UI (single-file components, composables, API clients).
- `src-tauri/` contains the Rust/Tauri backend and app wiring.
- `resources/` stores bundled defaults like `resources/config.yaml`.
- `public/` and `src/assets/` contain static assets and global styles.
- `docs/` and `prototypes/` provide design notes and experiments.

## Build, Test, and Development Commands
- `bun install`: install JS dependencies.
- `bun run dev`: run the Vite frontend in isolation.
- `bun run tauri dev`: run the full Tauri app with hot reload.
- `bun run build`: type-check (`vue-tsc`) and build the frontend.
- `bun run tauri build`: produce production desktop builds.
- `bun run preview`: serve the built frontend locally.

## Coding Style & Naming Conventions
- TypeScript with `strict` enabled; avoid unused locals/params (`tsconfig.json`).
- Use existing indentation (tabs in `.vue` files, 2 spaces in JSON).
- Vue composables live in `src/composables/` and are named `useX.ts`.
- API clients live in `src/api/` (e.g., `mihomo.ts`, `tauri.ts`).
- Prefer descriptive component names (`ProfilesManager.vue`) and kebab-case for CSS utility classes.

## Testing Guidelines
- No automated tests are present yet; add them alongside new features when feasible.
- If adding tests, place them near the code under test (e.g., `src/.../__tests__/`).
- Keep test names descriptive (e.g., `useMihomo.test.ts`).

## Commit & Pull Request Guidelines
- Git history is minimal (single “first commit”), so no formal convention is established.
- Use short, imperative commit messages when adding new work (e.g., `add profiles view`).
- PRs should include: a concise summary, testing steps, and screenshots for UI changes.
- Link relevant issues or design docs when applicable.

## Configuration & Local Setup Notes
- The app expects the Mihomo binary and config in per-OS app data paths (see `README.md`).
- Default Mihomo API endpoint is `http://127.0.0.1:29090`; avoid hard-coding secrets in configs.
