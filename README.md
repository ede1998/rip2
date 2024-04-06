# rip (Rm ImProved)

`rip` is a command-line deletion tool focused on safety, ergonomics, and performance.  It favors a simple interface, and does *not* implement the xdg-trash spec or attempt to achieve the same goals.

> This version is a fork:
>
> 1. Kevin Liu's original `rip`, which has been left untouched since 2020.
> 2. ⇒ [@StandingPadAnimation](https://github.com/StandingPadAnimations) fork which adds a few features.
>
> I'm doing a fork of this fork, half to learn rust, and half to implement customizations.
> I also like to understand software I use, especially when that software is not actively developed.

Deleted files get sent to the graveyard (`/tmp/graveyard-$USER` by default, see [notes](#notes) on changing this) under their absolute path, giving you a chance to recover them.  No data is overwritten.  If files that share the same path are deleted, they will be renamed as numbered backups.

`rip` is made for lazy people.  If any part of the interface could be more intuitive, please open an issue or pull request.

## Installation

You can install this package from source with:

```bash
$ cargo install --git https://github.com/MilesCranmer/rm-improved.git
```

No binaries are made available at this time.

## Usage

```text
USAGE:
    rip [FLAGS] [OPTIONS] [TARGET]...

FLAGS:
    -d, --decompose    Permanently deletes (unlink) the entire graveyard
    -h, --help         Prints help information
    -i, --inspect      Prints some info about TARGET before prompting for action
    -s, --seance       Prints files that were sent under the current directory
    -V, --version      Prints version information

OPTIONS:
        --graveyard <graveyard>    Directory where deleted files go to rest
    -u, --unbury <target>       Undo the last removal by the current user, or specify some file(s) in the graveyard.  Combine with -s to restore everything printed by -s.

ARGS:
    <TARGET>...    File or directory to remove
```

Basic usage -- easier than rm

```bash
$ rip dir1/ file1
```

Undo the last deletion

```bash
$ rip-u
Returned /tmp/graveyard-jack/home/jack/file1 to /home/jack/file1
```

Print some info (size and first few lines in a file, total size and first few files in a directory) about the target and then prompt for deletion

```bash
$ rip -i file1
dir1: file, 1337 bytes including:
> Position: Shooting Guard and Small Forward ▪ Shoots: Right
> 6-6, 185lb (198cm, 83kg)
Send file1 to the graveyard? (y/n) y
```

Print files that were deleted from under the current directory

```bash
$ rip -s
/tmp/graveyard-jack/home/jack/file1
/tmp/graveyard-jack/home/jack/dir1
```

Name conflicts are resolved

```bash
$ touch file1
$ rip file1
$ rip -s
/tmp/graveyard-jack/home/jack/dir1
/tmp/graveyard-jack/home/jack/file1
/tmp/graveyard-jack/home/jack/file1~1
```

-u also takes the path of a file in the graveyard

```bash
$ rip -u /tmp/graveyard-jack/home/jack/file1
Returned /tmp/graveyard-jack/home/jack/file1 to /home/jack/file1
```

Combine -u and -s to restore everything printed by -s

```bash
$ rip -su
Returned /tmp/graveyard-jack/home/jack/dir1 to /home/jack/dir1
Returned /tmp/graveyard-jack/home/jack/file1~1 to /home/jack/file1~1
```

### Emacs

```elisp
(setq delete-by-moving-to-trash t)
(defun system-move-file-to-trash (filename)
  (shell-command (concat (executable-find "rip") " " filename)))
```

## Notes

**Aliases.**

You probably shouldn't alias `rm` to `rip`.  Unlearning muscle memory is hard, but it's harder to ensure that every `rm` you make (as different users, from different machines and application environments) is the aliased one.

What I instead recommend is aliasing `rm` to an echo statement that simply reminds you to use `rip`:

```bash
alias rm="echo Use 'trash' instead of rm."
```

**Graveyard location.**

If you have `$XDG_DATA_HOME` environment variable set, `rip` will use `$XDG_DATA_HOME/graveyard` instead of the `/tmp/graveyard-$USER`.

If you want to put the graveyard somewhere else (like `~/.local/share/Trash`), you have two options, in order of precedence:

  1. Alias `rip` to `rip --graveyard ~/.local/share/Trash`
  2. Set the environment variable `$GRAVEYARD` to `~/.local/share/Trash`.

This can be a good idea because if the graveyard is mounted on an in-memory filesystem (as `/tmp` is in Arch Linux), deleting large files can quickly fill up your RAM.  It's also much slower to move files across filesystems, although the delay should be minimal with an SSD.

**Miscellaneous. **

In general, a deletion followed by a `--unbury` should be idempotent.

The deletion log is kept in `.record`, found in the top level of the graveyard.
