# Coding Agent Profile

## Identification
- **Name**: Antigravity IDE (Gemini Assistant Engine)
- **Class**: Autonomous & Interactive Coding Agent

## Repository Access Model
- **Direct Workspace Access**: Reads and writes files directly within the local workspace repository.
- **Terminal Integration**: Executes sandboxed shell commands (Cargo toolchain, Git commands, directory management).
- **Session State**: Maintains conversational and task context during development cycles.

## Permitted Operations
- **Repository Analysis**: Inspect directory hierarchy, search patterns, parse abstract syntax and config files.
- **File Manipulation**: Create, update, and refactor source code, configuration, and documentation artifacts.
- **Validation**: Invoke `cargo check`, `cargo build`, and `cargo test` to verify build integrity and test passes.
- **Version Control Assistance**: Inspect `git status` and `git diff`, propose branch workflows and commit structures.
