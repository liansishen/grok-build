# AGENTS.md

## Release Requirements

Every release must include a written changelog entry before the release is published.

The changelog entry must:

- identify the exact release tag and release date;
- summarize upstream syncs and fork-specific changes;
- list user-visible features, fixes, breaking changes, and important compatibility notes;
- state whether new user-visible fields or messages require internationalization, and record the translation status;
- be reviewed against the commits included since the previous release;
- be reflected in the GitHub Release notes rather than relying only on automatically generated comparison links.

The release process must not be considered complete until the changelog is committed and pushed with the release source, the remote CI/release workflow succeeds, and the published GitHub Release contains the corresponding written notes.
