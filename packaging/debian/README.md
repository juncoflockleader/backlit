# Debian Packaging

This directory is the Debian packaging skeleton for Ubuntu Server/headless Ubuntu installs.

Near-term tasks:

- Add source package metadata.
- Split binary packages by compositor, shell, session, portal, settings, desktop meta-package, and dev tools.
- Install session files from `packaging/sessions`.
- Install user/system units from `packaging/systemd`.
- Add package-level smoke tests.

