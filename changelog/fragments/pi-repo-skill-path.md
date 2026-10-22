### Fixed

- Repository-targeted skill installs now put Pi skills in `.pi/skills`, where Pi discovers them, while user installs keep `.pi/agent/skills`. Reinstalling migrates only managed, non-newer resources from the old repository path; dry-run previews the migration without writing.
