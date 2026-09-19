# histlint

A linter for shell history files. It reads `~/.bash_history`,
`~/.zsh_history`, or any file in that shape, and flags lines that are worth
a second look: credentials typed straight into a command, destructive
deletes, and scripts piped from the network directly into a shell.

Shell history is one of the least protected places secrets end up. It's
plain text, it's rarely encrypted, it gets backed up, synced, and grepped by
whatever's convenient. This isn't a replacement for a secrets manager, it's
a smoke detector for the history file you already have.

## Building

No dependencies, so this only needs a stable Rust toolchain:

```
cargo build --release
```

The binary ends up at `target/release/histlint`.

## Usage

```
histlint ~/.bash_history
```

Sample output:

```
142: [danger] plaintext-credential: command embeds a credential after `password`
    mysqldump --password=hunter2 mydb > backup.sql
201: [danger] pipe-to-shell: downloaded script is piped directly into a shell without inspection
    curl -sSL https://example.com/install.sh | bash
310: [danger] destructive-rm: recursive delete targets `/`
    sudo rm -rf /

3 finding(s)
```

Exit status is `0` when there are no findings and `1` otherwise, so it's
usable in a pre-commit hook or CI step over an exported history file.

### Reading from stdin

With no file argument, or with `-` in its place, histlint reads from stdin.
That makes it possible to lint your live session history without writing it
to disk first:

```
history | histlint
```

The `history` builtin prefixes each line with its entry number (`  501  git
status`); histlint strips that off before running the rules against it.

### JSON output

`--json` prints the same findings as a single JSON array, one object per
finding, instead of the text report:

```
histlint --json ~/.zsh_history
```

```json
[{"line":142,"rule":"plaintext-credential","severity":"danger","message":"command embeds a credential after `password`","command":"mysqldump --password=hunter2 mydb > backup.sql"}]
```

Line numbers refer to the line in the source file the finding starts on,
which matters for zsh's extended history format where one logical command
can be a continuation of several physical lines.

## What it checks today

- `plaintext-credential` — `password=`, `token=`, `secret=` and similar
  assignments on the command line, `-p` passwords with no space (mysql/psql
  style), and `user:pass@host` basic auth embedded in a URL.
- `pipe-to-shell` — `curl`/`wget` output piped into `sh`, `bash`, or `zsh`.
- `eval-remote` — `eval` combined with a network fetch.
- `destructive-rm` — recursive `rm` targeting `/`, `~`, `$HOME`, or `*`.

## Config

Some rules don't fit every project's history (a warning about `-p` passwords
is noise if your team only ever uses `.pgpass`). Drop a `.histlintrc` in the
directory you run histlint from to turn rules off:

```
# .histlintrc
disable = pipe-to-shell
disable = eval-remote, destructive-rm
```

Lines starting with `#` are comments, blank lines are ignored, and a rule
name that doesn't exist is a load error rather than a silent no-op. Pass
`--config <path>` to load a config from somewhere other than the current
directory:

```
histlint --config ci/histlintrc ~/.bash_history
```

With no `.histlintrc` present and no `--config`, all rules run.

## History formats

Plain one-command-per-line files (bash's default) and zsh's extended
history format (`: <epoch>:<duration>;<command>`, including backslash
line continuations) are both handled by the same parser.

## License

MIT, see [LICENSE](LICENSE).
