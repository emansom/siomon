# AUR Packaging - Agent Context

## Purpose

Package build template for the Arch User Repository (AUR). The CI workflow
overwrites version fields and checksums at build time.

## Key Sources

- `packaging/aur/README.md` — how the build process works, version format,
  and local testing instructions.
- `.github/workflows/publish-aur.yml` — the workflow implementation.
- `packaging/aur/PKGBUILD` — the template itself.
- `PACKAGING.md` — one-time setup guide for secrets, SSH keys, and AUR
  account configuration.
