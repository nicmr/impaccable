Additional features:
- package subgroups / "meta" group type
- "ignore" group type
- add ability to override pacman flags both as cli flags and in config file
  ```toml
  [pacman]
  install_flags = ""
  remove_flags = ""  ```
- add ability to add groups as subgroups of others (or not? too complicated)
- low priority: add groups info, creation, deletion cli commands
- `config mv-packagedir` command

Bugs
- plan / sync is listing packages that are already installed :/
- - handle differences from reference packagelist better, e.g. eos https://github.com/endeavouros-team/calamares/blob/19bce10d8e1d6637b0c303d8807f5a7e6bd38491/data/eos/scripts/remove-ucode#L8

Refactors:
- move active target management behind active target manager that tracks file location
- possibly rename target to hosts(?), as target is also used as another name for package in pacman
- check if dialoguer's interact_opt is required or just interacting

- contain 

- weird things about package set
- why is sed explicitly installed if it's already in base?
  - probably have to ask at eos github