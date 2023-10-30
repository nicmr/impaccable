Additional features:
- package subgroups / "meta" group type
- "ignore" group type
- add ability to override pacman flags both as cli flags and in config file
  ```toml
  [pacman]
  install_flags = ""
  remove_flags = ""  ```
- add ability to add groups as subgroups of others (or not? too complicated)

Bugs
- plan / sync is listing packages that are already installed :/

Refactors:
- move active target management behind active target manager that tracks file location
- possibly rename target to hosts(?), as target is also used as another name for package in pacman
- is it even worth it to do the generics dance with IntoIterator to pass to pacman?
- check if dialoguer's interact_opt is required or just interacting
- remove diff (?) add as --no-groups flag to plan maybe? alternative ideas: --simple

- weird things about package set
- why is sed explicitly installed if it's already in base?