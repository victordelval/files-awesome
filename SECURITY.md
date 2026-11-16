# Security policy

Files Awesome edits files on your disk, so data safety and the webview sandbox
are treated as security boundaries. The model is described in
[docs/adr/0001-stack.md](docs/adr/0001-stack.md#security-model).

## Reporting a vulnerability

Please **do not open a public issue**. Use GitHub's private reporting instead:
*Security → Report a vulnerability* on this repository.

Especially in scope:

- The webview reading or writing a path the user did not pick.
- Any way for document content (links, images, HTML) to run script, reach the
  network, or call Tauri commands.
- Saves that can lose or corrupt data (non-atomic writes, silent overwrites,
  changed bytes in untouched content).

Only the latest release is supported while the project is pre-1.0.
