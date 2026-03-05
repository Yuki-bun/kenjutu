## Development Commands

```bash
# typecheck tauri frontend
pnpm check

# Generate ts bindings for tauri desktop app
make gen

# Cargo commands are available from the root of this repository
cargo build ....
```

## Code Style

- No semicolons
- Newtype wrappers for IDs: `ChangeId`, `CommitId` in `crates/kenjutu-types`
- Minimize comments. Explain _why_, not _what_. If code needs a comment to explain what it does, rewrite the code instead.

## Version Control

- Use jujutsu to create commits
- Large changes should be split into multiple commits with clear messages
- Do not create a commit that fixes a previous commit; instead modify the old revision with jujutsu commands
- Never use git commands to create or modify commits, always use jujutsu
- Before running jujutsu commands always check the current state with `jj log`
- Run appropriate check commands in Makefile before committing, such as `make check-rust`
- For a sufficiently large change, in the plan stage document stage at which a commit will be created.
- Each commit should pass all the lints and tests.
