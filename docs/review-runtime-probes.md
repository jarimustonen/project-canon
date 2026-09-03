# Runtime probes for `project-canon review`

`review` is static-only by default. It executes a target only when the caller supplies
`--run <binary>`:

```sh
project-canon review --run ./target/debug/example-tool --json .
```

`--assume-defaults` remains the explicit static-only mode and cannot be combined with
`--run`. Runtime probes inspect CLI surfaces, so `--run` requires `--profile cli`.

## Safety contract

Runtime review invokes exactly the path supplied to `--run`. It does not search for or build a
binary. Each invocation uses a direct process call, never a shell, with null stdin, captured output,
a 1 MiB capture limit, and a three-second timeout. On the supported macOS and Linux targets, a timeout kills the child's
process group so ordinary descendants cannot keep running or hold capture pipes open. Missing,
non-executable, hanging, or crashing targets produce `could-not-probe`; they are never counted as a
pass or a conformance gap.

The probe argument vectors are fixed and read-only. They cover:

- §2: a deliberate unknown-command error plus the structured help exit-code map
- §8: `config path --json` and `config show --json`
- §10: `version --json`
- §14: `--help --json`
- §15: `skill list --json`; its read-only capability metadata must declare Claude, pi, and Codex,
  canonical per-runtime selection, explicit/default `all`, native layouts, `--target`, and
  non-interactive safety flags (the probe never invokes `skill install`)
- §16: `skill print <listed-name> --json`
- §17: version metadata, skill-list metadata, and printed skill frontmatter synchronization
- §18: `doctor --json <repo>` without a fix flag

The runner never invokes `new`, `skill install`, or a `doctor --fix` surface. Canon requirements
that need mutation, source inspection, signal injection, TTY comparison, or human judgment remain
manual checks.

## JSON contract

The review payload remains schema version 1 and includes a stable `runtime_probe` object:

```json
{
  "runtime_probe": {
    "enabled": true,
    "binary": "/absolute/path/to/example-tool",
    "timeout_ms": 3000,
    "outcomes": [
      {
        "id": "canon.s10",
        "status": "pass",
        "message": "version --json carries schema, compatibility, provenance, and skill metadata"
      }
    ]
  },
  "summary": {
    "could_not_probe": 0
  }
}
```

Each runtime outcome status is exactly `pass`, `gap`, or `could-not-probe`. A `gap` also appears in
`findings[]` as `kind: "confirmed-gap"` and may have a staged issue command. A
`could-not-probe` appears as `kind: "could-not-probe"`, has no staged command, and increments
`summary.could_not_probe`. Passing rows stay in `runtime_probe.outcomes`; they do not become
findings. With no `--run`, `enabled` is false, `binary` is null, and `outcomes` is empty.
