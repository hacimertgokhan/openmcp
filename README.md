# OpenMCP

OpenMCP is a Tauri + Vue desktop app and Node CLI for managing OpenCode MCP servers.

## Commands

```bash
npm install
npm run tauri dev
npm run build
npm run cli -- list
```

## What it manages

- Reads OpenCode config candidates such as `~/.config/opencode/opencode.json`, `~/.config/opencode/opencode.jsonc`, `~/.opencode.json`, and launch-project configs.
- Enables/disables MCP entries by writing `enabled: true/false`.
- Removes MCP entries after creating a `.bak-<timestamp>` backup.
- Installs curated catalog MCP definitions globally or into a project `opencode.json`.
- Copies an existing global MCP into a selected project.

Environment values are not shown in the UI; only environment key names are displayed.
