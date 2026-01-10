# PR Manager

A desktop application for efficiently managing and reviewing GitHub pull requests locally.

## Features

### Repository Management
- **Browse GitHub Repositories** - View all repositories accessible to your GitHub account
- **Link Local Repositories** - Connect your local repository clones to enable offline diff viewing
- **Quick Navigation** - Jump directly to any repository or GitHub URL from the app

### Pull Request Viewing
- **List Open PRs** - See all open pull requests for any repository
- **PR Metadata** - View PR number, title, author (with avatar), and description
- **Branch Information** - See base and head branch details for each PR
- **GitHub Integration** - Direct links to view PRs on GitHub

### Commit Exploration
- **Commit History** - Browse all commits included in a pull request
- **Commit Details** - View commit messages, descriptions, and SHA identifiers
- **Expandable Messages** - Full commit descriptions accessible via tooltips

### Diff Viewing
- **Unified Diff Display** - View all file changes in a clean, syntax-highlighted format
- **File Change Status** - See which files were added, modified, deleted, renamed, copied, or type-changed
- **Addition/Deletion Counts** - Track lines added and removed per file
- **Line-by-Line Changes** - Color-coded additions (green) and deletions (red) with line numbers
- **Binary File Detection** - Automatic handling of binary files
- **Expandable Files** - Collapse/expand individual file diffs for focused review

### Review Tracking
- **Mark Files as Reviewed** - Checkbox to track which files you've reviewed in a PR
- **Persistent Review State** - Review status saved locally and persists across sessions
- **Progress Tracking** - Easily see which files still need review

### User Experience
- **Fast Performance** - Local diff generation for instant file viewing
- **Clean Interface** - Card-based UI with organized sections
- **Reload Functions** - Refresh repositories and PRs with a single click
- **Error Handling** - Clear error messages and validation

## Getting Started

1. Create a GitHub personal access token with repository access
2. Save your token to `~/.config/pr-manager/token` (Linux/macOS) or `%APPDATA%\pr-manager\token` (Windows)
3. Launch the application
4. Browse your repositories
5. Select a repository and link your local clone (via file picker)
6. View and review pull requests

## Requirements

- GitHub account with personal access token
- Local git repository clones (for viewing diffs)
