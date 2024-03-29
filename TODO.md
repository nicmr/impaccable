Medium prio features:
- check command to just verify config
- init command? what does the program currently do when package folder is missing?
- package subgroups / "meta" group type (if not too complicated)
- "ignore" group type
- add ability to override pacman flags both as cli flags and in config file
  ```toml
  [pacman]
  install_flags = ""
  remove_flags = ""
  ```
- add config subcommand to dump computed package dir
- use logging instead of println, add relevant log in library
- detailed config error type that gives hint where in the config file the error occurs
- document usage of relative / absolute paths

Low prio features:
- add various small commands for scripting: e.g. groups info, creation, deletion cli commands
- `config mv-packagedir`  command

Bugs
- handle differences from reference packagelist better, e.g. cpu-specific ucode on eos https://github.com/endeavouros-team/calamares/blob/19bce10d8e1d6637b0c303d8807f5a7e6bd38491/data/eos/scripts/remove-ucode#L8 (-> with ignore groups?)

Refactors:
- move active target management behind active target manager that tracks file location
- possibly rename target to hosts/machines, as target is also used as another name for package in pacman
- check if dialoguer's interact_opt is required or just interact is fine
- error handling: create error implementation per use case instead? https://kazlauskas.me/entries/errors

Upstream:
- eos package set
  - why is sed etc. explicitly installed if it's already in base?
    - probably have to ask at eos github