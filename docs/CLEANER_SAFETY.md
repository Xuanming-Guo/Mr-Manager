# Cleaner safety model

Mr Manager Cleaner is a quarantine transaction, not a delete shortcut. Scanning is read-only. Quarantine, restore, and purge are separate user actions with different safety and confirmation requirements.

## Candidate categories

Deterministic low-risk rules cover recognized dependency/build caches such as `node_modules`, framework caches, Python caches, and Cargo `target` directories adjacent to a `Cargo.toml`. Conventional `dist`, `build`, `out`, virtual environments, large logs, archives, and large files require stronger review because their names or sizes do not prove they are reproducible.

Cleaner never proposes `.git`, `.env*`, credential/key files, databases, source files, or ordinary project assets. Duplicate repository detection is not implemented and folder-name similarity is never cleanup evidence.

Every candidate records its canonical path, selected root, category, reason, confidence, risk, byte estimate, file count, practical lock state, regeneration guidance, kind, and a metadata fingerprint. Nothing is selected automatically.

## Scan boundary

- The user must choose one to eight explicit roots.
- Drive roots, user-profile roots, Windows, Program Files, ProgramData, AppData roots, and Mr Manager quarantine storage are rejected.
- Roots may not overlap.
- Traversal is capped at 24 levels, 100,000 visited entries, and 5,000 candidates.
- File contents are not loaded during discovery. Candidate identity uses relative paths, file sizes, modification times, and entry types.
- Symlinks, junctions, mount points, and other Windows reparse points are skipped and never followed.
- Every traversed path is canonicalized and must remain below its selected root.
- Cancellation returns a safe partial result that cannot create a cleanup plan.
- Inaccessible items produce per-path issues rather than widening scope or aborting unrelated roots.

## Immutable plan and quarantine

The backend keeps a bounded server-side scan cache. Plan creation accepts candidate IDs only from that evidence, persists the full reviewed plan in SQLite, sorts it deterministically, and allows execution once. The typed confirmation phrase states the item count.

Immediately before each move, Cleaner re-canonicalizes the root and item, rechecks every component for links/reparse points, and recalculates size, count, kind, and identity fingerprint. A changed item stays in place and is recorded as failed.

Quarantine prefers an atomic rename. Copy fallback is allowed only across filesystems, only after the destination filesystem is identified and has the required free space plus a margin. The copy is verified before the original is removed. If original removal fails, the manifest records a partial state: the original remains and the verified managed copy may be purged separately.

The manifest is created transactionally with the plan before filesystem mutation and updated after every item. It records original and quarantine paths, category/reason, size/count, timestamps, verification method, restore/purge state, and safe failure details. There is no automatic purge; retention is indefinite until an explicit purge.

## Restore and purge

Restore uses the exact original path when it is free. If occupied, Cleaner refuses to overwrite and reports a server-generated sibling path. The user may explicitly choose **Restore beside existing**. Successful restore is verified and recorded.

Purge operates only inside canonical Mr Manager quarantine storage. It is irreversible, requires the exact item-specific `PURGE <item-id>` phrase, revalidates the managed path, rejects links/reparse points, and recursively removes only that reviewed quarantine item. No generated shell or deletion command is used.

Quarantine usually does not reclaim filesystem space because the managed copy remains available for restoration. The UI states this directly. A whole-manifest purge is available only after quarantine and requires `PURGE MANIFEST <manifest-id>`; the backend still resolves and revalidates each eligible managed item independently and persists the manifest after every attempt.

## Failure recovery

SQLite manifests are durable in the application-data directory. `inProgress`, `partial`, and `failed` item states identify operations needing review after an I/O error or interruption. Cleaner never silently retries against a changed source. The safe recovery path is to inspect the manifest, close applications holding locks, and run a fresh scan before creating another plan.

Destructive tests use only temporary synthetic fixtures. The acceptance test quarantines and restores a synthetic `node_modules` tree byte-for-byte and path-for-path while asserting that a sibling source file remains unchanged.
