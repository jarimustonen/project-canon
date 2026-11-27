### Fixed

- `review --runtime-probes`: after a target execution failure, each later `could-not-probe` row
  now says "not attempted after target execution failure:" once, followed by the original
  failure reason, instead of repeating the prefix once per earlier row.
