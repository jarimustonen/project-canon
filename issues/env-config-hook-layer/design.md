# Design: env config/hook layer

The first **verb-independent seam**. `doctor`, `new`, and `review` all inherit it, so the shape
matters more than the values. Per ADR 0009 §2/§5/§6: the two-layer model
(`resolved(repo) = BASE ∪ PROFILE[archetype]`) is untouched — this adds an *orthogonal* config
seam, it does not change model semantics.

## The seam

One resolved struct + a sparse override layer + a three-step resolution order, all in
`crates/project-canon-core` as a new `env` module. It is **orthogonal to `Model`**: a verb reads
both a `Model` (what conformance means) and an `EnvConfig` (where *this* environment's repos,
account, and registration live). They never fold into each other.

```text
EnvConfig::resolve(&[&file_layer, &env_layer])
  = builtin_defaults()          // step 1 — the single source of the homebase values, one place
      .apply(file_layer)        // step 2 — a parsed config file (lowest override)
      .apply(env_layer)         // step 3 — process env vars (highest precedence)
```

Precedence is the AI-first CLI §8 order **flag > env > file > default** minus the flag rung
(no CLI surface exists yet). `resolve` takes an **ordered slice** of layers, so a future verb
adds the flag rung by appending it — `resolve(&[&file, &env, &flags])` — with no change to the
module. Strict validation (§1/§4) lives at each source's *parse edge* (`from_env_vars` rejects a
malformed bool, a blank scalar, or an empty list element, echoing the offending variable), so the
merge itself is infallible.

## Types (all in `env.rs`, `Layer`-pure — zero I/O in core)

- **`EnvConfig`** — the fully-resolved values a verb consumes:
  - `gh_account: String` — the gh account (default `"jarimustonen"`).
  - `repo_root: String` — the `~/Sources/<name>` location convention's base (default `"~/Sources"`);
    `repo_location(name) -> "<repo_root>/<name>"` computes the convention in **one** place.
  - `family_tools: BTreeSet<String>` + `repo_overrides: BTreeMap<String,String>` — the family-repo
    map. Known tools resolve by convention (`repo_location`); an override pins an off-convention
    repo. `family_repos()` materializes `tool → path` **on demand**, so overriding `repo_root`
    re-derives every path (no stale absolute paths baked at default-time). This is the single
    source the `cli-canon` skill's hard-coded map should read from (homebase cutover: separate issue).
  - `tw: TwRegistration { enabled: bool, projects_conf: String }` — tw / `projects.conf`
    registration (default enabled, `~/.config/tw/projects.conf`).
  - `workmux_emoji_prefix: Option<String>` — the `.workmux.yaml` emoji prefix. Default `None`
    (portable: no glyph); homebase sets its own via config — exactly the non-portable specific we
    externalize.
  - `ci_release: CiReleaseHook { pattern: Option<String> }` — **documented extension point** for
    the future `hauis` CI release pattern. Unpopulated at v0 (`None`); the field exists so the
    seam is stable when `hauis` lands.
- **`EnvConfigLayer`** — a sparse all-`Option` override. `apply` merges field-by-field; the
  family-repo override map *extends* (add/repoint one tool without redeclaring the set).
- **`EnvConfigError::InvalidBool { var, value }`** — §1/§4 strict validation: a malformed
  `..._ENABLED` value errors echoing the var and the bad value, never a silent coerce.

## Layer sources live at the I/O edge, not in core

Core stays zero-dependency and I/O-free (matching the crate's existing "serde is a deferred
seam" discipline). The two override layers are *produced* at the edge and handed to core:

- **env layer** — `EnvConfigLayer::from_env_vars(&BTreeMap<String,String>)` is **pure** (maps
  `PROJECT_CANON_*` vars → layer fields, `§8` env-mirrors-flag naming). The CLI passes
  `std::env::vars()`; tests pass a synthetic map. Returns `Result` (invalid bool → error).
- **file layer** — the on-disk config *format* and its read are the CLI's concern (a future
  `config` surface / `--config` flag). Core merges whatever `EnvConfigLayer` the edge parsed;
  today the resolution-order tests construct the file layer in-memory. Deferred like serde:
  added when a verb reads a file, not speculatively.

## How a verb consumes it

At the CLI edge a verb builds the config once, then passes the resolved `EnvConfig` into any
core operation that needs an environment specific:

```rust
let env_layer = EnvConfigLayer::from_env_vars(&std::env::vars().collect())?;
let file_layer = /* future: parse --config / default path; empty today */ EnvConfigLayer::empty();
let cfg = EnvConfig::resolve(&[&file_layer, &env_layer]);
let home = std::env::var("HOME")?; // the ~ expansion needs the home dir → stays at the edge
// new:    scaffold at EnvConfig::expand_home(&cfg.repo_location(name), &home); gh cfg.gh_account
// doctor: probe EnvConfig::expand_home(&cfg.family_repo("issuectl")?, &home); check .workmux prefix
```

Config paths are `~`-relative **config strings**; `EnvConfig::expand_home(path, home)` is the
pure half of tilde resolution (the edge supplies `$HOME`, core stays I/O-free) so every verb
inherits one expansion. Scaffold dimensions (`scaffold.rs`) stay **abstract** ("has an
`AGENTS.md`", never a path) — the *concrete* values now come from `EnvConfig`, keeping core free
of hardcoded paths/accounts/hosts.

## Deferred (not this issue)

- **Tri-state patches** — a sparse layer cannot yet *clear* an optional value or *remove* a
  lower layer's override. Env (the only wired source) cannot express a clear anyway; the need
  first bites when the config-**file** layer lands, so an `Override<T>` / `Option<Option<T>>`
  patch belongs to that issue.
- **`repo_overrides` via env** — per-tool overrides are file/flag-only (no fragile
  `TOOL=path,…` env grammar); `FAMILY_TOOLS` covers the env case (replace the known set).
- **Homebase-value defaults** — the defaults intentionally *preserve today's homebase behavior*
  (single-source, nothing breaks for Jari); portability = every value overridable + core
  hardcoding none. `workmux_emoji_prefix`/`ci_release` default to `None` only because there is
  no homebase value to carry. A future `EnvConfig::portable_defaults()` preset could invert this
  if a second environment adopts the tool.
