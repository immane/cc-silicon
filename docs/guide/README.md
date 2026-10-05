# Human Guide: Reading Order

This guide is for people who want to understand the project without
first reading the compiler task protocol. Each page answers one
question. Read them in order; skip anything you do not need.

| Order | Page | Question it answers |
|---|---|---|
| 1 | [Project overview](PROJECT_OVERVIEW.md) | What are we building, and why this odd chip-based design? |
| 2 | [Architecture](ARCHITECTURE.md) | What are the parts, and how do they talk to each other? |
| 3 | [Compilation walkthrough](COMPILATION_WALKTHROUGH.md) | What happens to a small C program on its way through the planned compiler? |
| 4 | [Project status](PROJECT_STATUS.md) | What works today, and what is still only a plan? |
| 5 | [Roadmap](ROADMAP.md) | What comes next, and what is blocking it? |
| — | [Glossary](GLOSSARY.md) | What does each strange word actually mean? |

The glossary is a reference: open it whenever a term looks unfamiliar,
then return to where you were.

## What this guide is not

- It is not a task assignment. To implement something, read
  [tasks/README.md](../tasks/README.md) instead.
- It is not the contract. Frozen identifiers, wire tags, and field
  permissions live in `compiler/` and the task packages.
- It does not claim the compiler works. Anything not yet built is
  labeled **planned**, and anything not yet measured is labeled
  **unverified**.

## If you remember only three things

1. The repository holds a **generic execution framework** plus a
   **frozen compiler contract foundation**. A working C compiler on top
   of them is **planned, not implemented**.
2. **Chips never write shared state directly.** Each chip submits a
   change request; one commit path checks and applies it.
3. **Frozen means the shape of the agreement is fixed**, not that the
   feature is finished. Details are in
   [Project status](PROJECT_STATUS.md).
