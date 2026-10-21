# Rolebit architecture

## Boundaries

Rolebit is a single Tauri desktop process. React owns presentation and interaction state; Rust owns persistence, secrets, scheduling, paid-provider access, external page retrieval, extraction, and ingestion orchestration. The two sides communicate through explicit Tauri commands and the `ingestion-progress` event.

No localhost service or hosted database is involved.

## Main modules

| Module | Responsibility |
|---|---|
| `src-tauri/src/commands.rs` | Typed UI command surface and input validation boundary |
| `src-tauri/src/db.rs` | Migrations, repositories, FTS queries, transactions, upserts, and run telemetry |
| `src-tauri/src/query.rs` | Site normalization and deterministic Cartesian query generation |
| `src-tauri/src/provider.rs` | Bright Data adapter, retries, response parsing, and mock SERP results |
| `src-tauri/src/extractor.rs` | Public-URL validation, guarded redirects, JSON-LD, HTML fallback, and normalization |
| `src-tauri/src/decision.rs` | OpenAI Decisions API page classification and privacy-filtered profile scoring |
| `src-tauri/src/ingestion.rs` | Concurrency, cancellation, deduplication within a run, progress, and completion status |
| `src-tauri/src/secret.rs` | Windows Credential Manager access for the API key |
| `src-tauri/src/lib.rs` | Application lifecycle, tray, scheduler, recovery, and command registration |
| `src/lib/api.ts` | Typed frontend command client and clearly isolated browser preview adapter |
| `src/pages/*` | Product workflows and database-backed exploration surfaces |

## Database

Migration 1 creates:

- `application_settings`
- `search_items`
- `search_queries`
- `ingestion_runs`
- `query_executions`
- `discovery_results`
- `jobs`
- `job_sources`
- `job_discoveries`
- `extraction_attempts`
- `job_revisions`
- `jobs_fts` with synchronization triggers

SQLite starts with foreign keys enabled, WAL journal mode, normal synchronization, and a ten-second busy timeout. Ingestion writes use atomic transactions; canonical job upserts use an immediate transaction to serialize concurrent identity decisions.

Migration 5 stores Decisions API classification confidence, overall profile fit, skill, experience, seniority, and location scores, plus matching errors. Match scores are part of the job content hash, so a profile-driven score change is retained in the revision history.

Timestamps are persisted as UTC RFC 3339 strings. Pagination and filters execute in SQLite. FTS5 searches title, company, and description without loading the full library into React.

## Ingestion state

Only one run may own the process-wide running flag. A run creates query executions concurrently up to the user setting. Each result URL is deduplicated within that run before extraction and page classification, then committed independently so one failed query does not discard other successful work.

The Rust extractor gathers evidence for every fetched HTML page without making a local job-versus-generic decision. Decisions API classifies the page as a specific posting, careers page, contact page, company page, or other. Non-postings are recorded as filtered discoveries. Only a specific posting is sent for profile scoring; a scoring error is stored with the job, so it does not discard the posting.

Final states are:

- `completed`: every query completed.
- `partial`: useful work completed but at least one query failed.
- `failed`: no query completed successfully.
- `cancelled`: the user requested cancellation.
- `interrupted`: Rolebit recovered a run left active by an earlier process exit.

Cancellation is checked between queries, pages, and candidate extractions. Completed database operations remain consistent.

## Identity and change detection

The source key is `<source-domain>:<external-id>` when a reliable external ID exists; otherwise it is the canonical normalized URL. A SHA-256 hash covers normalized posting fields and profile-match results rather than raw HTML. Changes create a revision snapshot and update the canonical record without overwriting non-empty values with missing extraction results. Tracking status is never changed by ingestion.

## Scheduler

The scheduler checks once per minute while the process is running. It compares the last scheduled or completed run with the configured interval. Starting a scheduled execution records the schedule timestamp immediately, which limits catch-up behavior to one execution after a long absence. The same overlap guard is shared by UI, tray, startup, and scheduled triggers.

## Security decisions

- API secrets use the operating-system credential store.
- OpenAI Decisions API calls originate in Rust; the OpenAI key is never exposed to the webview.
- Classification sees page evidence only. Profile scoring starts after a specific posting is identified and sends an allowlist of professional fields, excluding names, contact details, street address, profile URLs, project URLs, credential URLs, and local record IDs.
- The CSP permits only packaged content, Tauri IPC, data images, and the local asset protocol.
- Provider access occurs in Rust, keeping secrets out of the webview.
- Candidate URLs and every redirect are restricted to public HTTP/HTTPS destinations.
- Remote HTML is parsed as data; scripts are never executed.
- The frontend cannot issue arbitrary network requests under the configured CSP.
