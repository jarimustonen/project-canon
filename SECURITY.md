# Security Policy

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues,
discussions, or pull requests.**

Report privately using **GitHub's
[Private Vulnerability Reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)**:
open the repository's **Security** tab → **Report a vulnerability**.

Include, as far as you can: the affected version or commit, the component and
threat surface, reproduction steps or a proof of concept, and the impact you
observed.

## Threat Surface

The areas most worth a researcher's attention:

- **Runtime probes** (`review --run <binary>`) — the one place project-canon
  executes an external program. It is designed to invoke only the explicitly
  named binary, directly (no shell), with read-only probe arguments, null stdin,
  captured output, and a per-invocation timeout, and to parse the probed
  binary's output as untrusted data. A way to make `--run` execute anything
  else, reach a shell, or turn probe-output parsing into more than a report is a
  vulnerability.
- **Prebuilt binaries and installers** — releases ship binaries via GitHub
  Releases, a shell installer, and a Homebrew tap. Supply-chain concerns about
  the release pipeline or artifact integrity are in scope.
- **Scaffolding and skill installation** (`new`, `skill install`) — these write
  files; writes escaping the declared target directory or agent skill layout
  would be a vulnerability.

## What to Expect

- We will acknowledge your report as soon as we can.
- We will confirm the issue, assess its severity, and keep you informed of
  progress.
- We practice coordinated disclosure: please give us a reasonable window to
  release a fix before any public disclosure. We will credit you unless you
  prefer to remain anonymous.

## Safe Harbor

We consider good-faith security research conducted under this policy to be
authorized. We will not pursue or support legal action against researchers who
act in good faith, avoid privacy violations and service disruption, and give us
a reasonable time to respond before disclosure.
