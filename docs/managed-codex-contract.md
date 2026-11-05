# Managed Codex connector — 0.2

The implementation follows TypeScript protocol contracts generated locally by the installed Codex CLI **0.159.2**, using `codex app-server generate-ts --out <directory>`. It starts that native CLI through stdio. It does not inspect private desktop-app databases or attach to existing desktop work.

Discovery (`model/list`, `thread/list`) is separate from explicit task execution (`thread/start`, `turn/start`). The connector requests standard service, user-reviewed on-request approvals, and a read-only sandbox by default. It verifies the returned approval routing and sandbox type before submitting a prompt. Workspace-write mode requires opting in.

One managed connection can be attached. Task intent is saved before sending. No pending request is replayed after restart. Missing delivery acknowledgement or loss of a connection before a source-reported terminal outcome is represented conservatively as unknown. An identical unknown request requires a separate acknowledgment before resubmission.

Supported interactions are command approval, bounded complete file-change approval, and non-secret user questions. Only accept-once and decline are supported. Session-wide approvals, persistent write-root grants, unreviewable actions, secret questions, and unknown provider callbacks are rejected. Newer file-change events invalidate earlier patch approvals. Pending decisions are bound to their live RPC identities, session and turn.

Turn completion, command exit codes, changed paths and answer text are source reports. No percentage, ETA, successful tests or independent correctness is inferred. Output is byte bounded and may be truncated. Local history allows up to 50 records and 1 MiB serialized payload; active records reserve 64 KiB each, so the actual capacity depends on record sizes. Backups include receipts and clear live decisions on restore.

Cancel requests interrupt the current turn and wait for a source-reported interrupted outcome. End connection is separate and explicitly releases the runtime. A completed turn retains its connection until that action because background work is not independently verified. Closing only the panel leaves the Rust core and attached runtime running. Normal Quit is blocked while attached.

Verification uses a PATH-injected, separately compiled protocol fixture in isolated test data. The fixture never executes commands or contacts a model. It is excluded from the distributed executable; only its source is included in the source archive. Passing these tests establishes client behavior against the fixture, not live model/provider compatibility. A real inference task, provider-side approval round trip, network interruption and background-job lifecycle remain device/provider acceptance work.
