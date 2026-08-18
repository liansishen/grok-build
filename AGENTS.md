# AGENTS.md

## Release Requirements

Every release must include a written changelog entry in the repository's existing changelog before the release is published.

The authoritative changelog is [`crates/codegen/xai-grok-shell/CHANGELOG.md`](crates/codegen/xai-grok-shell/CHANGELOG.md). New entries must follow its existing format and section names, including the version heading and the applicable `Features`, `Bug Fixes`, `Breaking Changes`, and `Performance` sections. Do not create a separate root-level changelog for fork releases.

Each changelog entry must:

- identify the exact release version and release date;
- summarize user-visible features, fixes, breaking changes, and important compatibility notes included since the previous release;
- state whether new user-visible fields or messages require internationalization and record the translation status when applicable;
- be reviewed against the commits included since the previous release;
- be reflected in the GitHub Release notes rather than relying only on automatically generated comparison links.

The release process is complete only after the changelog is committed and pushed with the release source, remote CI succeeds, and the published GitHub Release contains the corresponding written notes. Do not create or publish a new version unless the user explicitly requests it.
