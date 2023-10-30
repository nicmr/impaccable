# impaccable

A mildly declarative pacman wrapper for Arch Linux.

## Key features

- store your installed packages in git
- easily readable toml files
- group your packages logically, reference groups
- store multiple target configurations

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

### AUR package

WIP

## Setting up a new system

```bash
yay -S impaccable
# clone your repo
git clone https://git.example.com/your/dotfiles .dotfiles
cd .dotfiles
# set up dotfiles using your dotfile manager, e.g. dotter
# dotter deploy
impaccable sync
```
