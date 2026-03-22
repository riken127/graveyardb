# GraveyardDB Documentation

This directory contains the scope and architecture notes for the current repository state.

GraveyardDB's core lifecycle is `event -> transition -> snapshot`. Every append path described in these docs assumes transition metadata is required on each event.

## Documentation Structure

* [Scope and Purpose](SCOPE.md): What the project currently does and what is still out of scope.
* [Architecture](ARCHITECTURE.md): Current module layout, data flow, and operational notes.
* [Release Process](../RELEASE.md): Semantic versioning, conventional commits, and changelog flow.
* [Changelog](../CHANGELOG.md): Release notes and unreleased changes.

## Getting Started

Refer to the root [README](../README.md) for setup and local run instructions.
