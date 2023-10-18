Features:
- fix missing save bug. Update: idk what this means.
- import: import installed packages into your configuration
  - interactive (default) / all : select packages
- add ability to override pacman flags both as cli flags and in config file
- move active target management behind active target manager that tracks file location
- remove diff (?) add as --no-groups flag to plan maybe? alternative ideas: --simple
- add flags to limit plan to not / only show destructive / additive


Bugs
- actually start using the selected root package groups instead of whole package set for uninstall, etc.
- plan / sync is listing packages that are already installed :/

Refactors:
- possibly rename target to hosts(?), as target is also used as another name for package in pacman
- is it even worth it to do the generics dance with IntoIterator to pass to pacman?