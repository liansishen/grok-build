# AGENTS.md

## Release Requirements

Every release must include a written changelog entry in the repository's existing changelog before the release is published.

The authoritative changelog is [`crates/codegen/xai-grok-shell/CHANGELOG.md`](crates/codegen/xai-grok-shell/CHANGELOG.md). Fork release entries must distinguish two kinds of content:

- For upstream synchronization, follow the structure and concise summary style of `v1.0.5-fork.1`: identify the upstream monorepo and Source-Revision, use an `上游更新` section, record the relevant upstream changes and previously synced upstream items when needed, then include `本 Fork`, verification, release assets, and a full comparison link.
- For fork-added features or problem fixes, follow the detailed user-facing style of `v1.0.5-fork.3`: explain behavior, user impact, edge cases, compatibility, persistence or display details when relevant, internationalization status, verification coverage, release assets, and a full comparison link.

Keep the existing version heading and release-note structure. Do not replace detailed release notes with a short code-only summary, and do not create a separate root-level changelog for fork releases.

Each changelog entry must:

- identify the exact release version and release date;
- summarize changes included since the previous release;
- state whether new user-visible fields or messages require internationalization and record the translation status when applicable;
- be reviewed against the commits included since the previous release;
- be reflected in the GitHub Release notes rather than relying only on automatically generated comparison links.

The release process is complete only after the changelog is committed and pushed with the release source, remote CI succeeds, and the published GitHub Release contains the corresponding written notes. Do not create or publish a new version unless the user explicitly requests it.
