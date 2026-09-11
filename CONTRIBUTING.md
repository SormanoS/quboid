# Contributing to Quboid

Thank you for considering a contribution. This document explains the licensing
terms that contributions are accepted under. Please read it before opening a
pull request.

## Licensing of contributions

Quboid is released under the GNU General Public License, version 3, and the
author also offers it under separate commercial terms. Keeping both options open
requires that the author holds the rights to every line in this repository, so
contributions are accepted under the agreement below.

**By submitting a contribution to this project, you agree to the following.**

1. **Grant of copyright licence.** You grant Samuele Sormano a perpetual,
   worldwide, non-exclusive, royalty-free, irrevocable copyright licence to
   reproduce, prepare derivative works of, publicly display, publicly perform,
   sublicense and distribute your contribution and any derivative works of it,
   **under any licence terms, including proprietary ones**. You keep the
   copyright in your contribution and remain free to use it however you like,
   including in other projects.

2. **Grant of patent licence.** You grant Samuele Sormano and every recipient of
   the software a perpetual, worldwide, non-exclusive, royalty-free, irrevocable
   patent licence to make, have made, use, offer to sell, sell, import and
   otherwise transfer the work, covering only those patent claims you own or
   control that are necessarily infringed by your contribution alone or by the
   combination of your contribution with this project.

3. **Right to grant.** You confirm that each contribution is your original work,
   or that you have the right to submit it under these terms. If your employer
   holds rights to work you create, you confirm you have permission to
   contribute, or that your employer has waived those rights.

4. **Third-party material.** If your contribution includes work that is not
   yours, you identify it, its source and its licence in the pull request. Only
   material under a licence compatible with the GPL v3 can be accepted.

5. **No warranty.** Contributions are provided as is, without any warranty,
   express or implied.

The project may be distributed under the GPL v3, under commercial terms, or
both. Your contribution stays available under the GPL v3 in every published
version that contains it.

## Why this arrangement

Quboid is free software and is meant to stay that way. The additional grant
exists so the author can offer commercial licences and build paid extensions on
top of the same code. Without it, a single contribution would make that
impossible for the whole project.

If you would rather not agree to these terms, you are still very welcome to open
issues, report bugs, discuss designs and maintain your own fork under the GPL
v3.

## Submitting a pull request

Before opening a pull request, make sure the checks pass:

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Note that `cargo test` runs a guard that fails if separately licensed types,
pages or runtime logic appear in this tree. If it fires, your change is touching
material that belongs to another distribution; open an issue instead.

## Where tests live

A test goes in `crates/<crate>/tests/` unless it needs access to private items.

Those integration tests link the crate the way a consumer does, so they only see
the public API and they document it. Reach for a `#[cfg(test)] mod tests` inside
`src/` only when the test genuinely calls a private function or inspects private
state: `config.rs` exercises its migration helpers that way, and the Win32
adapters do the same. A test that compiles from `tests/` belongs in `tests/`.

Sign off each commit with `git commit -s` to record your agreement with the
terms above.
