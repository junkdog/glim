## glim

[![Crate Badge]][Crate] [![Deps.rs Badge]][Deps.rs]

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

### Installation

```
cargo install glim-tui
```

### Running

To use glim, you'll need a GitLab personal access token (PAT) for authentication with the GitLab API.
Be aware that this PAT is stored in plain text within the configuration file. If you start glim
without any arguments and it hasn't been set up yet, the program will prompt you to enter the PAT
and the GitLab server URL.

```
$ glim -h
A TUI for monitoring GitLab CI/CD pipelines and projects

Usage: glim [OPTIONS]

Options:
  -c, --config <FILE>      Alternate path to the configuration file
  -p, --print-config-path  Print the path to the configuration file and exit
  -h, --help               Print help
  -V, --version            Print version
```

#### Multiple GitLab servers

There is currently no support for multiple GitLab servers in the configuration file. The interim
solution is to use the `--config` flag to specify a different configuration file, e.g. 
`glim --config glim-corporate.toml` or `glim --config glim-personal.toml`.



  [Crate Badge]: https://img.shields.io/crates/v/glim-tui.svg
  [Crate]: https://crates.io/crates/glim-tui
  [Deps.rs Badge]: https://deps.rs/repo/github/junkdog/glim/status.svg
  [Deps.rs]: https://deps.rs/repo/github/junkdog/glim
