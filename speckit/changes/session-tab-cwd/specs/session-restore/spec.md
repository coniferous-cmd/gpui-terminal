---
create-time: 2026-10-28 22:08:34
---

## ADDED Requirements

### Requirement: Session launch cwd survives restoration independently of tab folders

The snapshot SHALL store a required launch cwd for every session separately from each tab's saved folder. Restoration SHALL retain that session cwd unchanged and restore existing tabs using the existing per-tab restoration policy. New tabs added after restoration SHALL use the retained session cwd. A session cwd SHALL be nonempty and rooted; filesystem availability SHALL be checked when launching a new tab, not used to rewrite the saved cwd during restore.

#### Scenario: Session and tab directories differ

- **WHEN** a session with launch cwd `/projects/luna` and tabs saved in `/tmp` and `/var/log` is saved and restored
- **THEN** its existing tabs restore using their respective folders and a subsequently added tab starts in `/projects/luna`.

#### Scenario: Session launch directory disappeared

- **WHEN** the saved session cwd no longer exists but existing tabs can be restored
- **THEN** those tabs restore normally, the session retains its unavailable cwd, and adding a new tab fails rather than changing that cwd.

### Requirement: Layouts without session launch cwd are incompatible

A snapshot containing any session without the required launch cwd, or with an invalid launch cwd value, SHALL be rejected as a whole through the existing invalid-snapshot path. The application SHALL log the reason and open the default home session rather than partially restoring that snapshot. It SHALL NOT infer a cwd from a tab, migrate the old layout, or silently default the missing field. A valid empty snapshot SHALL remain valid.

#### Scenario: Old layout has no session cwd

- **WHEN** the application loads an old snapshot whose sessions contain names, selections, and tabs but no launch cwd
- **THEN** the whole layout is dropped with a diagnostic and the normal default home session opens.

#### Scenario: Only one session is missing cwd

- **WHEN** a snapshot has several sessions and any one lacks its launch cwd
- **THEN** none of the snapshot's sessions are restored.

#### Scenario: Invalid cwd value

- **WHEN** a saved session cwd is empty, relative, null, or not a string
- **THEN** the snapshot is rejected with a diagnostic identifying the invalid field.
