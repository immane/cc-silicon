# Documentation

This repository keeps **one set of facts with two reading entrances**.

## For humans: understand the project

Start with the [human guide](guide/README.md). It explains, in plain
language with a worked example, what this project is building, how the
pieces fit together, what works today, and what comes next. It assumes
no prior knowledge of the task protocol or frozen contract internals.

- [Reading order and map](guide/README.md)
- [Project overview](guide/PROJECT_OVERVIEW.md)
- [Architecture](guide/ARCHITECTURE.md)
- [Compilation walkthrough](guide/COMPILATION_WALKTHROUGH.md)
- [Project status](guide/PROJECT_STATUS.md)
- [Roadmap](guide/ROADMAP.md)
- [Glossary](guide/GLOSSARY.md)

## For contributors and AI agents: do the work

Start with the [task index](tasks/README.md). Task packages, acceptance
plans, and contract proposals are the authoritative work orders. They
use exact interface names and frozen identifiers on purpose.

- [Task master plan](tasks/README.md)
- [Per-chip template](tasks/TASK_TEMPLATE.md)
- [Parallel execution rules](tasks/PARALLEL_EXECUTION.md)
- [Compiler development guardrails](tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md)

## Normative references

- [Paradigm specification](architecture/SILICON_PARADIGM_SPEC.md)
- [SFL contract](architecture/SFL_CONTRACT.md)
- [SFL schema draft](architecture/SFL_SCHEMA_DRAFT.md)
- [Design blueprint](design/ARCHITECTURAL_BLUEPRINT.md)
- [Getting started (framework API)](design/GETTING_STARTED.md)
- [Frozen contract artifact](../compiler/contracts/CONTRACT_VERSION)

## One rule against documentation drift

The guide **explains**; it does not define. When the guide and a
contract, task package, or code disagree, the contract, task package,
or code wins. The guide links to its sources instead of copying field
tables, so there is only one place where each fact lives.
