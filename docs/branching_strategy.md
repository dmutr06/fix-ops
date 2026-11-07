# Branching and commits

## Branches

`main` is always green. All work lands through a pull request.

Branch names, by how many Linear tickets the work covers:

- One ticket: `<ticket>/<short-title>`, e.g. `fix-28/staff-access`
- Two tickets: `<id1>-<id2>/<shared-title>`, e.g. `fix-27-28/staff-access`
- More: `<type>/<shared-title>`, e.g. `feat/repair-workflow`

Lowercase, kebab-case, titles shortened when long.

## Commits

Conventional Commits, imperative, at most 72 characters, one line, no body or trailers.

| Prefix | Use |
|---|---|
| `feat:` | New behavior |
| `fix:` | Bug fix |
| `test:` | Tests only (the failing tests of a TDD cycle) |
| `refactor:` | No behavior change |
| `docs:` | Documentation in `docs/` or the README |
| `spec:` | Requirements and contract in `spec/` |
| `logs:` | Gate and verification records in `logs/` |
| `chore:` | Build, CI, dependencies |

No ticket ids in commits; the branch carries them.

## Pull requests

- Title: `FIX-28: Staff access`, or `FIX-27, FIX-28: Staff access` for several tickets.
- Body: a short overview, then the related tickets at the end.
- CI (fmt, clippy, tests) must pass. PRs are squash-merged with a Conventional Commit title.
