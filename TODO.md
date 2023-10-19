Essential features:
- import: import installed packages into your configuration
  - interactive (default) / all : select packages

Additional features:  
- add ability to override pacman flags both as cli flags and in config file
  ```toml
  [pacman]
  install_flags = ""
  remove_flags = ""  ```

Bugs
- actually start using the selected root package groups instead of whole package set for uninstall, etc.
- plan / sync is listing packages that are already installed :/

Refactors:
- move active target management behind active target manager that tracks file location
- possibly rename target to hosts(?), as target is also used as another name for package in pacman
- is it even worth it to do the generics dance with IntoIterator to pass to pacman?
- check if dialoguer's interact_opt is required or just interacting
- remove diff (?) add as --no-groups flag to plan maybe? alternative ideas: --simple
