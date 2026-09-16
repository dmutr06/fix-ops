# Branching and Commit Strategy

## Branching Model
- `main`: stable line, tracks validated specifications and functional code.
- Feature branches: short-lived branches created from `main`.
  - Prefix `feat/<name>` for human-driven changes.
  - Prefix `agent/<name>` for coding agent proposals.
- Merges into `main` require validation against `/spec` before commit.

## Commit Message Convention
Commit messages strictly follow SDD prefixes:
- `spec:` updates to requirements and architecture documents in `/spec`.
- `feat:` domain and application logic in `/src`.
- `test:` unit and integration tests in `/tests`.
- `docs:` workflow and project guides in `/docs`.
- `logs:` agent profiles and audit records in `/logs`.
- `gate:` merge commits, conflict resolutions, and compliance checks.
