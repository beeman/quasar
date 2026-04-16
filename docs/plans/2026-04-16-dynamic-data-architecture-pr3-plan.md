---
title: "Dynamic Data Architecture + PR3"
type: architecture
date: 2026-04-16
---

# Dynamic Data Architecture + PR3

## Goal

Define one clear model for dynamic data in Quasar without collapsing distinct
concerns into one abstraction. Then implement:

- `#141` declaration-ordered instruction arg decoding
- `#156` repr-backed unit enum support in `QuasarSerialize`

This document is intentionally split into:

- what should be true now
- what PR3 should change
- what the next mutation-API redesign could look like

## Problem

Quasar currently has one good idea and one bad leak:

- good idea: account dynamic storage and mutation are optimized for Solana
  constraints
- bad leak: instruction arg decoding accidentally inherited an internal
  two-phase model that does not match the wire format

The architecture gets cleaner if we stop treating all "dynamic data" as one
thing.

## Dynamic Data Layers

### 1. InstructionWire

Instruction data is a transient ABI.

Properties:

- starts from raw `&[u8]`
- producer is the client
- consumer is one instruction invocation
- declaration order is the ABI
- dynamic fields must be compact on the wire

Invariants:

- args serialize in declaration order
- args deserialize in declaration order
- `String<N>` and `Vec<T, N>` use prefix + live payload only
- no fixed-capacity instruction encoding
- no account-style guard or storage machinery in the instruction path

Consequence:

- `#141` should be fixed by replacing the current split
  `[all fixed][all dynamic]` decode plan with a single sequential cursor

### 2. AccountRead

Account reads interpret persistent program-owned storage.

Properties:

- starts from validated account data
- storage layout must remain stable on-chain
- repeated field access should be cheap
- fixed header and dynamic tail are acceptable

Invariants:

- discriminator and fixed header are validated first
- dynamic tail is parsed by walking from the tail start
- default dynamic accounts store live-length payload only
- read-only access should avoid copies

Consequence:

- `AccountRead` and `InstructionWire` can share decode primitives, but not the
  same semantic abstraction

### 3. AccountMut

Account mutation is a write-optimization problem.

Properties:

- writes should be batched
- realloc should happen at most once per mutation scope
- memmove/serialization should be amortized
- no allocator

Current invariants:

- default dynamic accounts store live-length payload on-chain
- mutation loads dynamic fields into stack-owned `PodString` / `PodVec`
- save happens once on explicit save or drop
- `fixed_capacity` is a separate mode with full inline ZC storage

Consequence:

- the current `DynGuard` model is a reasonable ergonomic default
- it should not define instruction ABI semantics

## What Is True Today

The current code still matches this split:

- default `#[account]` with `String<N>` / `Vec<T, N>` uses live-length on-chain
  storage
- `#[account(fixed_capacity)]` disables the dynamic tail path and places those
  fields inline in the ZC struct
- `as_dynamic_mut()` still loads stack-owned pod buffers and flushes them once
  on drop

That means PR3 should preserve the account model and only fix the instruction
model.

## PR3 Scope

### `#141`

Replace macro-generated instruction decoding with a single declaration-ordered
cursor plan:

- fixed scalars / fixed structs decode from the current offset
- borrowed dynamic forms decode from the current offset
- direct dynamic `String<N>` / `Vec<T, N>` decode from the current offset
- offset always advances in source order

This intentionally keeps the compact live-length wire format.

### `#156`

Add repr-backed unit enums as scalar instruction args.

Rules:

- only unit enums
- require `#[repr(u8 | u16 | u32 | u64 | i8 | i16 | i32 | i64)]`
- encode/decode via the repr scalar
- invalid discriminants return decode errors

This should integrate into the same instruction-arg model rather than
introducing a parallel enum ABI.

## Non-Goals For PR3

- no redesign of account mutation APIs
- no forced unification of account storage and instruction wire internals
- no fixed-capacity instruction encoding
- no attempt to remove `DynGuard`

## Proposed Next-Step Account Mutation Architecture

The current mutation path is good for small and medium bounded dynamic fields,
but it ties stack usage to declared capacity. The next redesign should make
that trade explicit.

### Tier 1: `DynGuard` stays as the ergonomic default

Use when:

- capacities are small or moderate
- ergonomics matter more than absolute stack efficiency
- one-save-on-exit semantics are desirable

Properties:

- stack-owned pod containers
- edits feel like normal local mutation
- one final flush

### Tier 2: `DynWriter` for large-capacity or stack-sensitive workloads

Use when:

- capacities are large
- stack budget matters
- caller can provide the replacement dynamic fields explicitly

Concept:

- keep a mutable view over the dynamic tail region
- parse offsets lazily
- edits are recorded as logical operations against a field
- one final `commit()` performs the minimal required layout rewrite

Rough API:

```rust
let mut dynw = account.as_dynamic_writer(payer, rent_lpb, rent_threshold);

dynw.set_name("new-name")?;
dynw.set_tags(new_tags)?;
dynw.commit()?;
```

Semantics:

- caller stages replacement values for the dynamic fields it wants to rewrite
- no full-capacity pod containers are materialized on the stack
- `commit()` computes final total size once, reallocs once if needed, and
  rewrites the dynamic region once

Implementation strategy:

- treat this as an explicit dynamic-section writer, not as the ergonomic default
- avoid copying full declared capacity onto the stack
- keep `DynGuard` as the default for small/medium in-place ergonomic edits

Important point:

The first implementation can be a writer rather than a fully general mutable
view. That still gives Quasar the right architectural split:

- `DynGuard` for ergonomic save-on-drop edits
- `DynWriter` for explicit low-level dynamic rewrites with one commit

## Shared Low-Level Primitives

These layers should share primitives, not semantics.

Good shared pieces:

- prefix readers/writers
- bounds-checked cursor advancement
- typed live-length decoders for strings and vecs
- encode helpers for repr-backed enums

Bad shared pieces:

- reusing account storage layout as instruction ABI
- reusing `DynGuard` concepts for instruction decode
- forcing grouped abstractions that hide whether a path is transient ABI or
  persistent storage

## Why This Is Better

- fixes the real bug in `#141` instead of documenting around it
- keeps instruction data compact
- preserves the current performance win of batched account writes
- gives Quasar a clean story for "small ergonomic dynamic mutation" and
  "large stack-sensitive dynamic mutation"
- prevents future confusion between wire format and storage format

## Review Gate

Before any code changes:

1. Review this architecture
2. Confirm PR3 scope is limited to instruction decode + enum support
3. Defer account mutation redesign to a dedicated follow-up PR/issue
