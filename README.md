# flowbit

A TUI tool for viewing GitHub Project v2 boards in your terminal.

## Features

- **Board View** — Kanban-style columns based on your Project's Status field
- **List View** — Flat table with repo, number, title, status, assignee, and more
- **Detail Pane** — View issue/PR metadata at a glance
- **Filter** — Search by title, label, assignee, kind, and status
- **Default Filters** — Pre-configure startup filters in config
- **Browser Integration** — Open any issue/PR in your browser with `Enter`
- **Offline-friendly** — Cached data displayed when API is unavailable
- **Vim Keybindings** — Navigate with hjkl

## Installation

```bash
cargo install --path .
```

### Requirements

- Rust 1.85+ (edition 2024)
- A GitHub Personal Access Token with `read:project` scope

## Configuration

Create `~/.config/flowbit/config.toml`:

```toml
[github]
# Token is resolved in order: GITHUB_TOKEN env var → this field
token = "ghp_your_token_here"

[project]
owner = "your-org-or-user"
number = 1
status_field = "Status"    # Name of the single-select Status field

[ui]
default_view = "board"     # "board" or "list"

[filter]
# Optional: filters applied on startup (Esc to clear)
# assignee = "your-username"
# labels = ["bug"]
# kind = "issue"
# status = "In Progress"
```

Or set the token via environment variable:

```bash
export GITHUB_TOKEN="ghp_your_token_here"
```

## Usage

```bash
flowbit
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` | Move left (Board: previous column) |
| `l` / `→` | Move right (Board: next column) |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `Tab` | Switch between Board and List |

### Actions

| Key | Action |
|-----|--------|
| `Enter` / `o` | Open in browser |
| `r` | Refresh data |
| `/` | Start filter input |
| `Esc` | Clear filter / close modal |
| `?` | Show help |
| `q` | Quit |

### Filter Syntax

```
fix login                  # title substring match
label:bug                  # exact label match
assignee:alice             # exact assignee match
is:pr                      # kind filter (is:pr / is:issue)
status:done                # exact status match
#123                       # issue/PR number
label:bug assignee:alice   # AND combination
```

## Architecture

```
src/
  main.rs            # Entry point + tokio runtime
  app.rs             # App state + event loop
  config.rs          # TOML config loading
  api/               # GitHub GraphQL client
  model/             # Domain types (ProjectItem, Filter)
  views/             # Board, List, Detail, Help views
  widgets/           # Filter input, Toast, Status bar
```

## License

MIT
