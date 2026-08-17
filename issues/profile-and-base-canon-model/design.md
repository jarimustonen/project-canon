# Design: base-canon + archetype-profile model (v0: `cli` profile only)

Issue: `profile-and-base-canon-model` · epic: `project-canon-v0` · ADR 0009 §1/§4/§6.

This is the **architecture keel** for project-canon's two-layer conformance model. The
downstream verbs — `doctor` (machine gate), `new` (scaffold), `review` (audit) — all consume
the types defined here; keep it legible.

> **Scope discipline (a LIFT, not a greenfield canon).** v0 authors ONLY the `cli` profile,
> which already exists as `AGENTS-AI-FIRST-CLI.md` §1–§22 + the `cli-canon` skill. We
> *reference* those sections; we do not re-author their prose. `service` / `library` /
> `release` ship as **named-but-empty extension points**.

## 1. The two-layer model

Conformance for any repo is resolved from **two additive layers**:

```
resolved(repo) = BASE CANON  ∪  PROFILE[archetype-of-repo]
                 (repo-invariant)   (archetype-specific, additive)
```

- **Base canon** — dimensions that hold for *every* project-canon-managed repo regardless of
  archetype. Two origins: (a) what homebase's `create-project` already scaffolds for any repo
  (doc pattern, issue tracking, git hygiene, README, gitignore) and (b) the **repo-general**
  canon sections that are not tied to a specific CLI surface shape: **§10** (schema/versioning
  contract), **§15–§17** (companion-skill install/print/sync), **§22** (internal `core`/`cli`
  layout). These come from the AI-first CLI canon but are treated as repo-invariant because
  create-project applies them to every repo it makes.
- **Profile** — the *additive* section-set an archetype layers on top of base. A profile is
  **a named section-set + a probe registry** (§3). Only `cli` has content at v0.

**Additivity, not replacement.** A profile never removes or overrides a base dimension; it
only adds. Resolution is a set union deduplicated by dimension id. For a `cli` repo:

```
BASE      = { §10, §15, §16, §17, §22 } ∪ { doc-pattern, issue-tracking, git-hygiene, readme, gitignore }
PROFILE[cli] = { §1, §2, §3, §4, §5, §6, §7, §8, §9, §11, §12, §13, §14, §18, §19, §20, §21 }   (the 17 not rooted in base)
resolved(cli) = §1–§22  ∪  the 5 base scaffold dims
```

So `resolved(cli)` is exactly the §1–§22 citation surface **plus** the repo-invariant
scaffold dims — the done-criterion "the `cli` profile resolves to the §1–§22 section-set".
For a `service`/`library`/`release` repo today, `PROFILE` is empty, so `resolved = BASE`
only — that *is* the extension-point contract (an empty profile resolves to base + nothing).

### Why §15–§17 sit in base rather than in the `cli` profile

This is the one non-obvious routing call. The companion-skill sections read as "CLI-ish", yet
create-project installs a companion skill for *every* repo (issuectl's `issue` skill, etc.),
so skill install/print/sync is a repo-invariant obligation, not a CLI-surface one. We follow
the TODO's explicit seed list (§10, §15–§17, §22 → base) and record the tension here: if a
future non-CLI archetype proves it genuinely ships no skill, §15–§17 move to a per-archetype
conditional. Until then, base is the faithful lift of what create-project actually does.

## 2. Dimensions: the unit of conformance

Everything resolvable is a **`Dimension`** — a single conformance requirement with a stable
id, a severity, an applicability rule, a layer, a source, and a probe. A canon §N section is a
Dimension sourced from the canon; a create-project scaffold requirement is a Dimension sourced
from the scaffold. Unifying them lets base and profile be plain sets of dimension ids.

```rust
struct Dimension {
    id: &'static str,          // stable registry key, e.g. "canon.s10", "base.doc-pattern"
    title: &'static str,
    severity: Severity,        // Must | MustWhenApplies | Should
    applicability: Applicability,
    layer: Layer,              // Base | Profile(Archetype) — where the dim is *rooted*
    source: DimensionSource,   // Canon{section: u8} | Scaffold | Discovered
    probe: Probe,              // machine-shaped {effect, signal, command_hint, fail}
}
```

- **`id`** is the stable citation key (mirrors the "stable probe ids" the cli-canon skill's
  Phase-4 extraction wants). Canon sections keep their §N in `source: Canon{section}` and in
  the id (`canon.s10`); §N is a stable surface, never renumbered.
- **`Probe`** is the machine-shaped descriptor lifted from `cli-canon`'s
  `templates/conformance-probes.md` (`effect-class`, `signal`, `command_hint`, `fail`). We
  store the compact probe, **not** the section's multi-paragraph prose — the prose stays in
  `AGENTS-AI-FIRST-CLI.md`, referenced by `source: Canon{section}`. This is the "reference/
  registry, not a re-copy" rule.
- **`Layer`** records where a dimension is *rooted*. Base dims are `Layer::Base`; the 17
  cli-only sections are `Layer::Profile(Cli)`. The `cli` profile's *declared* section-set is
  its 17 profile-rooted sections; the base layer independently contributes the other 5, so the
  union is the full §1–§22. (A test asserts base ∪ profile[cli] canon-sections == 1..=22.)

## 3. A profile = a named section-set + a probe registry

```rust
struct Profile {
    archetype: Archetype,       // Cli | Service | Library | Release
    members: Vec<&'static str>, // the profile-rooted dimension ids it adds over base
}
```

The "probe registry" is not a second data structure — a profile's probes are the `Probe`s of
its member dimensions, looked up in the shared `DimensionRegistry`. Keeping one registry (base
dims + all profile dims) means the probe a `review` runs and the scaffold a `new` emits can
never drift from the section they belong to — the same single-source guarantee cli-canon gets
from sharing one probe table across its review/generate modes.

`Profile::is_extension_point()` returns `members.is_empty()` — the v0 state of
`service`/`library`/`release`.

## 4. The applicability questionnaire (characterize → applicable-sections)

Mirrors `cli-canon`'s `SKILL.md` "Characterize the tool" step. The questionnaire does **two**
jobs:

1. **Select the archetype** → picks which `Profile` layers onto base. (v0: only `cli` has
   content; the archetype question is a stub that will grow as archetypes are authored.)
2. **Gate the conditional sections** within the resolved set, via the **eight yes/no
   questions** lifted verbatim from cli-canon:

   | Q | Question | Gates |
   |---|---|---|
   | Q1 | More than one resource noun? | §6 *shape* (noun-verb vs flat) — §6 always applies |
   | Q2 | Resolves persistent config / a data root? | §8 |
   | Q3 | Any command creates/updates/deletes? | §11 |
   | Q4 | Any command runs >a few seconds / as a daemon? | §12 |
   | Q5 | Stamps `created`/`updated` or time-derived ids? | §19 |
   | Q6 | Owns human-editable on-disk records? | §20 |
   | Q7 | Scaffolds an on-disk home other commands need? | §21 |
   | Q8 | Results that can be large? | §13 |

**`Applicability`** is either `Always` or `Conditional(Question)`. Resolution walks the union
of base+profile members and stamps each with a status:

```rust
enum AppStatus {
    Applies,                              // in scope for evaluation (severity decides gating)
    NotApplicable { gated_by: Question }, // out of scope — records the question that gated it off
}
```

- `Always` → `Applies`.
- `Conditional(q)` → `Applies` iff the questionnaire answered `q = true`, else
  `NotApplicable { gated_by: q }`. Carrying the gating question lets a downstream `doctor`/
  `review` report "§20 n/a: Q6 = no" without re-deriving applicability. Note `Applies` is the
  *evaluation-scope* axis, not readiness: a `Should` dimension applies yet never gates.

So the **section-set** (membership) is the union; **applicability** is a per-section status on
top. `resolved(cli, all-conditionals-yes)` marks all §1–§22 `Applies`. With some conditionals
`no`, those sections are still *members* of the set but stamped `NotApplicable` — matching
cli-canon's rule that "a conditional section whose Applies-when doesn't hold is `n/a`, never a
failure."

**§6 note (reconciliation).** The cli-canon questionnaire table maps Q1→§6, but the probe
table says "§6 Applies: **always** (shape of the whole surface)". We follow the probe table:
§6 is `Always`; Q1 selects its *shape* (noun-verb for multi-resource, flat verb for
single-resource) rather than switching it on/off. `resolve` records the chosen shape on the
`Resolution` as `Option<SurfaceShape>` — `Some` only when an applicable §6 is present, so a
non-CLI (base-only) resolution reports `None` rather than a meaningless `FlatVerb`. §6 is never
marked `NotApplicable`.

## 5. Routing dimension-discovery candidates to base vs. profile

The canon grows (cli-canon §19+). When a new dimension-discovery candidate arrives, it must be
routed to a layer. The rule, encoded as `routing::suggested_layer` and enforced by a test:

- **Holds for every archetype** (a repo-general practice — doc pattern, changelog policy, MSRV
  gate, git hygiene) → **`Layer::Base`**.
- **Specific to one archetype's surface** (a CLI-only verb contract, a service-only health
  endpoint, a library-only semver-API gate) → **`Layer::Profile(that archetype)`**.
- **Recurs across ≥2 archetypes but not all** → base, with the non-applicable archetypes
  gating it off via a `Conditional` applicability (same mechanism as the eight questions).

A candidate is only admitted once it clears cli-canon's ≥2-tool recurrence bar; below that it
is a watch-list note, not a Dimension. The `DimensionSource::Discovered` variant tags dims that
entered this way, so provenance (canon vs scaffold vs discovered) stays inspectable — which is
what a later `doctor`/`review` needs to explain *why* a dimension is in scope.

## 6. Crate layout (§22 core/cli split — the family's canonical shape)

A Cargo **workspace**, per §22, because the model must be unit-testable without the CLI and
the downstream verbs embed the core:

```
Cargo.toml                              # workspace: core + cli members
crates/project-canon-core/              # the model — no clap, no I/O, dependency-light
  src/{lib,dimension,canon,scaffold,profile,questionnaire,resolve,routing}.rs
  tests/resolution.rs                   # done-criteria integration tests
crates/project-canon-cli/               # thin binary `project-canon` — verbs come LATER
  src/main.rs
```

- **`project-canon-core`** holds the whole model and is where the injected `Clock` (§19) will
  live when timestamped verbs land. Zero external deps at v0 (no serde/clap yet) — JSON
  serialization of a `Resolution` (§10 for `doctor`/`review`) is a **deferred seam**, added
  when a verb consumes it, not speculatively.
- **`project-canon-cli`** is deliberately minimal: it does not implement `new`/`doctor`/
  `review` (separate, blocked issues). `main` emits a one-line status pointing at those issues
  and a smoke summary from core (keeps the dependency real). Adding clap + the §2 error→exit
  map + §10 envelope belongs to the first verb, not here — half-implementing the CLI canon
  would be worse than an honest stub.

## 7. No homebase-specific paths (seam for `env-config-hook-layer`)

The base scaffold dimensions are declared **abstractly** — "the repo has a consolidated
`AGENTS.md` with a `CLAUDE.md` symlink", not `<personal-repo-root>/...` or a `projects.conf` path.
*Probing* an actual repo for these (and the homebase env specifics: `tw`, `gh` account,
`.workmux` emoji) is the job of the later `env-config-hook-layer` issue and its config/hook
layer. This model hardcodes **no** filesystem path, account, or host. The `Probe.command_hint`
strings are illustrative `$TOOL`-shaped hints (as in conformance-probes.md), not executed
here.

## 8. Post-review refinements & deferred model work

A multi-model review (`history/review-profile-base-canon-model.md`) and its triage
(`history/assessment-profile-base-canon-model.{json,md}`) ran on the first cut. The mechanical
fixes landed in this commit (canon-id zero-padding + out-of-range panic, duplicate-id guard,
§15 `SandboxWrite` reclassification, `Option<SurfaceShape>`, `NotApplicable { gated_by }`,
disjointness assert). Five architectural findings were confirmed but deliberately deferred as
spin-offs — they are the model's v1 hardening, each needing its own design:

1. **Base-membership re-routing** — §10/§15–§17/§22 in base make non-CLI archetypes inherit
   CLI obligations. Split "ships a skill" from "exposes a `skill` subcommand", or admit
   archetype-gated conditional base dims. **Gates the first non-CLI profile** (see §1's note).
2. **Escalating severity** — the three-tier `Severity` can't express §13's "SHOULD in general,
   MUST when large" (audit §14 too).
3. **Sub-requirement / probe-case granularity** — §2/§8/§22 bundle several obligations under
   one severity/probe; the skill's own deferred Phase-4 machine-registry work.
4. **Tri-state characterization** — unanswered questions fail open to `false`; a conformance
   gate needs `Unknown` + `AppStatus::Undetermined` before `doctor`/`review` rely on it.
5. **Extensibility + provenance** — validated `ModelBuilder`, owned/namespaced ids,
   `model_version` on `Resolution`, a `Discovered` authoring path, and
   `routing::suggested_layer → (Layer, Applicability)`.

## 9. What this issue does NOT build

- The `new` / `doctor` / `review` verbs (separate issues, blocked on this).
- JSON output, clap surface, the §2/§10 plumbing (arrive with the first verb).
- The config/hook layer and any real repo-probing (`env-config-hook-layer`).
- Content for `service` / `library` / `release` (named-but-empty by design).
</content>
</invoke>
