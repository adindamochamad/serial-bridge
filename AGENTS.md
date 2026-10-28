# Repository Guidelines

## Project Overview

Cross-platform desktop application that bridges serial-port hardware to a graphical interface, with assisted analysis of the incoming data stream. It is a Tauri 2 application: a SvelteKit front end compiled to static assets, wrapped by a Rust host process that owns device access.

Layout and module boundaries:

```
src/                    SvelteKit front end
└── lib/                components and logic
src-tauri/              Rust host process
├── src/
│   ├── main.rs         binary entry point
│   ├── lib.rs          library entry point and command surface
│   └── discovery.rs    serial port discovery
├── capabilities/       Tauri permission grants
├── tauri.conf.json     application and bundle configuration
└── Cargo.toml          Rust dependencies
static/                 assets copied verbatim into the build
presets/                shipped configuration presets
tests/                  Vitest suites for front-end logic
docs/                   demo scripts and release procedure
.github/workflows/      CI and release automation
```

The trust boundary sits between the two halves. Anything that touches a serial device, the filesystem, or the operating system belongs in `src-tauri/`, exposed to the front end as an explicit command. The front end must not assume access it has not been granted; `src-tauri/capabilities/` is what actually authorises each plugin permission.

The front end is built with `@sveltejs/adapter-static`, so there is no server runtime. Do not add server-side endpoints, server load functions, or anything that expects a Node process at runtime.

Runtime dependencies of note: Tauri plugins for the store, the opener, and the serial port, plus an AI provider SDK used by the assisted-analysis features.

## Source of Truth

| Concern | File |
|---|---|
| Front-end dependencies and scripts | `package.json`, `package-lock.json` |
| Rust dependencies and crate metadata | `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` |
| Application, window, and bundle configuration | `src-tauri/tauri.conf.json` |
| Granted plugin permissions | `src-tauri/capabilities/` |
| SvelteKit and adapter configuration | `svelte.config.js` |
| Build configuration | `vite.config.js` |
| Test configuration | `vitest.config.ts` |
| TypeScript configuration | `tsconfig.json` |
| Verification gate | `.github/workflows/ci.yml` |
| Release process | `.github/workflows/release.yml`, `docs/RELEASE_GUIDE.md` |
| Released change history | `CHANGELOG.md` |
| Required environment variables | `.env.example` |
| Files excluded from commits | `.gitignore` |

`.env.example` lists which environment variables exist. Real values live in a local environment file that `.gitignore` excludes, together with a `private/` directory. Never read, quote, log, or commit either.

`.github/workflows/ci.yml` is the authoritative definition of what "verified" means for this project. If a command is not in there, do not present it as the gate.

## Setup and Commands

The package manager is npm, evidenced by `package-lock.json`. CI uses Node 20.

```bash
npm ci                  # install exactly what the lockfile pins
npm run dev             # front end in the browser, without the desktop shell
npm run tauri dev       # full desktop application
```

Front-end verification, exactly as CI runs it:

```bash
npm run check           # svelte-kit sync then svelte-check against tsconfig.json
npm test                # vitest run
npm run build           # static build
```

Rust verification, exactly as CI runs it, from `src-tauri`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
```

Clippy runs with warnings denied, so a warning fails the build. Fix the cause rather than adding an allow attribute.

`npm run tauri build` produces distributable bundles. That is a release action; see `docs/RELEASE_GUIDE.md` and do not run it as a routine check.

## Working Principles

- Understand the relevant code before editing it.
- Prefer the smallest complete change.
- Preserve established project patterns.
- Fix root causes rather than symptoms.
- Keep main execution paths simple and explicit.
- Avoid speculative abstractions and unrelated refactors.
- Avoid unnecessary configuration.
- Do not add fallback or compatibility behavior without evidence it is needed.
- Do not leave dead code or unexplained TODOs.
- Do not submit changes you cannot explain and maintain.

Complexity has a cost. Minimalism is never a reason to remove security, validation, compatibility, observability, reliability, or defensive behavior that production requires. Hardware-facing code in particular needs its error paths kept intact.

## Repository Conventions

### Naming language — English is required

All new names must be in English. This applies to variables, constants, functions, methods, classes, types, interfaces, modules, components, files, and directories, on both the TypeScript and the Rust side.

It applies equally to all new database names should persistence be added: databases, schemas, tables, columns, views, indexes, constraints, sequences, migrations, and any other database object.

- Use clear, descriptive English terms that stay consistent with existing repository conventions.
- Do not mix languages inside a single new identifier.
- Do not translate or rename existing identifiers outside the scope of your task.
- Do not rename a Tauri command, an event name, a persisted store key, a preset filename, or an exported type merely to satisfy this rule. Command and event names couple the two halves of the application, and store keys and presets are read from data already on users' machines.
- If a change genuinely requires a rename, perform an impact analysis first, keep reading the old persisted key during a migration window, and document the change in `CHANGELOG.md`.

This rule applies to every new name even though at least one existing module in `src/lib/` is named in another language.

### Front end

- Svelte 5 is in use. Follow the reactivity style already present in neighbouring components rather than mixing idioms.
- Components live in `src/lib/` alongside the plain TypeScript modules they use. Keep parsing, transformation, and export logic in `.ts` modules so it can be unit-tested without a DOM; components should render and orchestrate.
- Shared types belong in the existing types module rather than being redeclared per component.
- `npm run check` runs `svelte-check`, which type-checks templates as well as scripts. Do not silence it with `any` or with `@ts-expect-error`.

### Rust host

- Expose capability to the front end as an explicit, narrowly scoped command. Do not widen a command's surface to make a caller simpler.
- Adding a plugin permission means editing `src-tauri/capabilities/`. Grant the narrowest permission that works, and never broaden the capability set as an incidental step.
- Keep `cargo fmt` clean; formatting is enforced in CI.
- Serial device access can fail, disconnect, or block. Propagate errors to the front end as values rather than panicking in the host process.

### Presets and persisted data

- Shipped presets live in `presets/` and are surfaced through the bundled-presets module. Adding a preset means adding data, not branching logic.
- Persisted settings go through the Tauri store plugin. Treat existing keys as a compatibility surface: read defensively, tolerate missing fields, and do not drop unknown keys.

## Testing and Validation

`vitest.config.ts` includes only `tests/**/*.test.ts`, runs in a Node environment, and disables globals, so tests import what they use explicitly. There are currently five suites covering parsing, profiles, agent tooling, webhook handling, and scale logic.

- Run the smallest relevant test first, then `npm test`, then `npm run check`, then `npm run build`.
- Run the three Cargo commands whenever you touch anything under `src-tauri/`.
- Add or update tests when behavior changes. Because the suite lives in `tests/` and imports from `src/lib/`, new logic is testable only if it sits in a plain module; that is a reason to keep it there rather than inside a component.
- There is no test coverage of the Rust host and no end-to-end coverage of the desktop shell. Changes to device handling, window behavior, or permissions must be verified by hand on a real device, and the verification steps stated explicitly.
- Use an existing suite as a reference for structure and assertion style.
- Do not disable tests, weaken assertions, or add allow attributes to make a check pass.
- Do not hide relevant warnings or errors.
- Report validation honestly. State which of the six CI commands you ran, and what could not be verified without hardware.

## Complexity and Performance

- Complexity has a cost; prefer cohesive, readable code.
- Avoid abstractions without concrete demand.
- Avoid new dependencies for marginal benefit, and weigh a Rust dependency especially carefully since it affects every bundled platform.
- Performance claims require a reproducible baseline. State how it was measured and what the trade-offs are.
- Do not accept disproportionate complexity for marginal gains.
- Start with the smallest useful experiment, then expand once it proves out.
- The hot path is a continuous inbound data stream. Prefer bounded buffers and batched rendering over per-sample reactivity, and measure with a real stream rather than a synthetic loop.

## Dependencies

- Reuse existing dependencies when they are adequate.
- Do not add dependencies without a demonstrated need.
- Use npm for the front end and Cargo for the host, following the existing manifests.
- Do not edit `package-lock.json` or `src-tauri/Cargo.lock` manually; let the package managers update them.
- Consider maintenance, security, license, and transitive cost.
- Keep runtime dependencies separate from tooling.

## Production Safety

- Do not access production resources.
- Do not read, display, log, or copy credentials. The AI provider key is a paid credential; treat it as the highest-risk secret here. Add new variables to `.env.example` by name only, never with a value.
- Never place a secret in front-end code. Everything shipped in the static bundle is readable by anyone who installs the application.
- Do not widen `src-tauri/capabilities/` without an explicit request. Every added permission expands what the application may do on a user's machine, and a desktop application has far more reach than a web page.
- Do not weaken or remove the security-relevant settings in `src-tauri/tauri.conf.json`.
- Serial commands are sent to physical hardware. Do not add, auto-trigger, or broaden a write path to a device without an explicit request; an unintended command can damage equipment or produce unsafe machine behavior.
- Do not remove confirmation steps in front of destructive or device-writing actions.
- Do not change authentication, authorization, or how persisted credentials are stored without impact analysis.
- Do not change public contracts without impact analysis. Tauri command names, event names, persisted store keys, and exported data formats are consumed by the other half of the application and by files already on users' machines.
- Do not perform a release. Tagging, bundling, signing, and publishing are governed by `.github/workflows/release.yml` and `docs/RELEASE_GUIDE.md`.
- The release version is declared in four places that currently disagree: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and the local package entry in `src-tauri/Cargo.lock`. None of them matches the most recent release tag. Do not guess a value. Establish the intended version with the maintainer, then update all four together and verify against `docs/RELEASE_GUIDE.md` before any release.
- Do not hand-edit generated files, build output, or bundle artifacts.
- Do not bypass hooks, tests, or safeguards, and do not modify CI to make a change pass.
- Request confirmation before high-risk actions.

## Scope and Change Discipline

- Change only the files the task requires.
- Preserve existing working-tree changes. Do not overwrite, revert, or stage another contributor's unfinished work, including untracked directories that are not part of your task.
- Keep refactors separate from behavior changes.
- Avoid formatting churn unrelated to the task. Run `cargo fmt` only on code you already touched.
- Do not edit dependency, generated, or build-output files without a valid reason.
- Do not commit or push unless explicitly asked.
- Never add files excluded by `.gitignore`, and never commit a local environment file or the private directory.
- Record user-visible changes in `CHANGELOG.md` when they ship.
- Summarize the changed files and validation results when finishing.
