## glim

![GitLab Pipelines](screenshots/gitlab_pipelines.png)

[![GitLab Projects](screenshots/gitlab_projects_thumbnail.png)](screenshots/gitlab_projects.png)
[![Pipeline Actions](screenshots/pipeline_actions_thumbnail.png)](screenshots/pipeline_actions.png)

A terminal user interface (TUI) for monitoring GitLab CI/CD pipelines and projects.
Built with [ratatui](https://ratatui.rs/).

### Prerequisites
- a terminal emulator with support for 24-bit color, e.g. [kitty](https://sw.kovidgoyal.net/kitty/)
- a GitLab personal access token (PAT) with `read_api` scope
- `libssl-dev` installed on your system

### Building
```
cargo build --release 
```



