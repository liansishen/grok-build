# Contributing

This repository does **not** accept external pull requests or unsolicited
patches.

SpaceXAI develops this software internally. The public tree is published for
source transparency and local builds under the terms of the Apache License,
Version 2.0 (see [`LICENSE`](LICENSE)).

## Security reports

Please report security issues through the process described in
[`SECURITY.md`](SECURITY.md). Do not open a public issue for vulnerabilities.

## Licensing of this source

By downloading or using this source, you agree that your use is governed by
the Apache License, Version 2.0. No contributor license agreement is offered
because external contributions are not accepted.

## Upstream synchronization

When synchronizing `xai-org/grok-build`, keep changes against upstream files small and contiguous:

- Do not reformat untouched upstream code, rewrite its comments, or reorder unrelated imports.
- Resolve comment, import, and formatting-only conflicts by taking the upstream version.
- Keep temporary Fork-only fixes in `LOCAL_PATCHES.md` with a matching `LOCAL-PATCH(<id>)` marker and a reversion condition.
- Run the detached-worktree rehearsal and marker audit before changing the formal branch.
