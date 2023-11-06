# impaccable

A gentle declarative pacman wrapper for Arch Linux.

Hint: This CLI is in active development, breaking changes may occur.

## Key features

- store your installed packages in git, like you'd store your dotfiles
- easily editable toml files
- group your packages logically
- reference the groups you want to have installed per machine

## Usage scenarios

1. Importing your installed packages step by step

```bash
mkdir ~/.config/impaccale && cd ~/.config/impaccable && git init
impaccable import
git commit -m "Added my system packages"
# add remote and push ...
```

2. resulting config example

```toml
# ~/.config/config.toml
package_dir = "packages"

[targets.dev_machine]
root_groups = ["programming"]

[targets.home_server]
root_groups = ["server-base"]
```

```toml
# ~/.config/impaccable/packages/mypackage.toml
[programming]
members = ["rustup", "helix", "code"]

[server-base]
members = ["nginx"]
```

3. Setting up a new machine

```bash
git clone https://git.example.com/my/impaccable-config ~/.config/impaccable
# select one of your configured machine types ('targets')
impaccable target set dev_machine
# install all packages
impaccable sync

# now use your favourite dotfile manager to get your configs
```


## Installation

### Install from source

```bash
# directly from git
cargo install --git https://github.com/nicmr/impaccable
```

```bash
# clone the repo
git clone https://github.com/nicmr/impaccable && cd impaccable
# install with cargo
cargo install --path .
```

