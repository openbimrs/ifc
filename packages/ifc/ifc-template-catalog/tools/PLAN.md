# catalog tooling plan

Status: implemented and verified.

## Work queue

- [x] `GEN-CLI` - explicit source/output CLI, no machine-specific default.
- [x] `GEN-HASH` - ordered relative-path-plus-content SHA-256.
- [x] `GEN-VALIDATE` - 420/93 sets and 2,550/257 typed children.
- [x] `GEN-ATOMIC` - temp write, decode verification, atomic rename.
- [x] `GEN-REPRO` - two clean runs are byte-identical.

## Completion log

Append command, digest, size, and reproducibility proof.

- Generator decoded temporary output before atomic rename and reproduced identical bytes in two independent runs.
