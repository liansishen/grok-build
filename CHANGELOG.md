# Fork Changelog

This changelog tracks releases published from the `liansishen/grok-build` fork.

## v1.0.5-fork.4 — 2026-08-18

### Release Process

- Added the fork release changelog and documented the requirement that every release include written changelog and GitHub Release notes.
- This documentation follow-up corrects the release process record for `v1.0.5-fork.4` without creating a new release.

### Upstream Sync

- Synced the upstream monorepo revision `d71f6e0c`.
- Git session metadata now reads the HEAD object ID directly from refs without loading the commit object, so session startup and file watching remain responsive when a repository has a missing or damaged commit object.
- Added regression coverage for unborn repositories, missing HEAD objects, status reporting, and checkout repair behavior.

### Fork Features

- Preserved per-request model metrics in the pager.
- Preserved live session usage updates and display behavior.
- Preserved recovery for malformed Responses terminal output.
- Preserved the fork's English and Simplified Chinese internationalization changes.

### Release Verification

- Remote GitHub Actions build passed for Linux x86_64 and Windows x86_64.
- Published binaries include `SHA256SUMS` checksums.
