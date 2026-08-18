# AGENTS.md

## Release Requirements

Every release must include a written changelog entry in the repository's existing changelog before the release is published.

The authoritative changelog is [`crates/codegen/xai-grok-shell/CHANGELOG.md`](crates/codegen/xai-grok-shell/CHANGELOG.md). New fork release entries must follow the established release-notes style used by the previous fork release: use Chinese user-facing prose, explain the behavior and relevant edge cases in detail, record internationalization status, record verification and release assets, and include a full comparison link. Keep the existing version heading and applicable project sections; do not replace the release notes with a short code-only summary.

Each changelog entry must:

- identify the exact release version and release date;
- summarize user-visible features, fixes, breaking changes, and important compatibility notes included since the previous release;
- state whether new user-visible fields or messages require internationalization and record the translation status when applicable;
- be reviewed against the commits included since the previous release;
- be reflected in the GitHub Release notes rather than relying only on automatically generated comparison links.

The release process is complete only after the changelog is committed and pushed with the release source, remote CI succeeds, and the published GitHub Release contains the corresponding written notes. Do not create or publish a new version unless the user explicitly requests it.
