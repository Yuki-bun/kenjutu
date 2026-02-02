# Revue

**Revue** is a desktop application for reviewing GitHub pull requests locally with a focus on per-commit review workflow.

## What is Revue?

Revue is a Tauri 2-based desktop application that allows you to view and review GitHub pull requests on your local machine. It provides an enhanced diff viewing experience with:

- **Per-commit review workflow**: Review changes commit-by-commit for better context
- **Local git integration**: Works directly with your local repositories
- **Syntax highlighting**: Beautiful code highlighting powered by Syntect
- **Review tracking**: Keep track of which files you've reviewed with persistent SQLite storage

## Technology Stack

### Frontend
- **React 19** with file-based routing via TanStack Router
- **TanStack Query** for efficient data fetching
- **shadcn/ui** for beautiful UI components
- **Octokit** for GitHub API integration
- **Tailwind CSS** for styling

### Backend
- **Rust** with Tauri 2 framework
- **git2** for local git operations and diff generation
- **SQLite** (via rusqlite) for persistent storage
- **Syntect** (via two-face) for syntax highlighting
- **tauri-specta** for type-safe IPC between Rust and TypeScript

## Prerequisites

- **Node.js** (with pnpm)
- **Rust** (latest stable version)
- **Git**

## Development Setup

1. Clone the repository:
```bash
git clone https://github.com/Yuki-bun/revue.git
cd revue
```

2. Install dependencies:
```bash
pnpm install
```

3. Generate TypeScript bindings for Tauri commands:
```bash
pnpm gen
```

4. Run in development mode:
```bash
pnpm tauri dev
```

## Development Commands

```bash
# Type checking
pnpm check

# Generate TypeScript bindings for Tauri commands
pnpm gen

# Build for production (takes time)
pnpm tauri build

# Linting
pnpm lint

# Fix linting issues
pnpm lint:fix

# Format code
pnpm fmt
```

## Architecture

### Data Flow

1. **GitHub API calls** happen in the frontend via Octokit
2. **Repository registry** (GitHub repo ID → local path) is stored using Tauri Store
3. **Local git operations** (diffs, commits) go through Tauri commands to the Rust backend
4. **Review tracking** persists to a per-repository SQLite database at `.git/revue.db`

### Key Directories

- `/src/routes` - Page components (file-based routing)
- `/src/components` - Shared React components
- `/src/hooks` - Custom React hooks
- `/src/lib` - Utility modules
- `/src-tauri/src/commands` - Tauri IPC command handlers
- `/src-tauri/src/services` - Business logic (auth, diff, git, highlight, review)

## Version Control

This project uses [Jujutsu](https://github.com/martinvonz/jj) for version control instead of Git for development:

```bash
# Create commits with jj
jj commit -m "Your commit message"

# For multiple commits, split changes into logical pieces
jj split
```

## License

[Add license information here]

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
