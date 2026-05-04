# Contributing to Locus

Thanks for helping improve Locus.

## Quick start

1. Fork the repository.
2. Create a feature branch from `main`.
3. Make focused changes with tests.
4. Run local validation:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

5. Open a pull request with a clear title and summary.

## Contribution guidelines

- Keep changes scoped to one concern per PR.
- Follow existing code style and naming.
- Add or update tests for behavior changes.
- Update docs (README, docs/architecture.md, or protocol docs) when behavior changes.

## Pull request checklist

- [ ] Builds successfully
- [ ] Tests pass
- [ ] Docs updated (if needed)
- [ ] No unrelated refactors

## Reporting issues

Use GitHub Issues for bugs and feature requests.
For security disclosures, see [SECURITY.md](SECURITY.md).
For community expectations, see [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).