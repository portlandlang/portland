# `System`

**Summary:** Ruby has no module for the host boundary — its syscall-touching functions are scattered across `Kernel`, `File`, `Process`, `ENV`, `IO`, and `Time`; Portland gathers every host-touching primitive under `System`, public but not published, and keeps Ruby's spellings on the wrappers written over it.

**Status:** decided ([ADR 0068](../adr/0068-2026-10-25-system-is-the-floor-module.md), 2026-10-25); `System.arguments` and `System.read_file` built on both implementations the same day with the bare placeholders retired, the rest of the first members in [#176](https://github.com/portlandlang/portland/issues/176).

## Ruby

There is no `System` constant. The functions that touch the operating system live wherever their receiver suggests: `File.read` and `File.write` on `File`, `Process.pid` and `Process.clock_gettime` on `Process`, `ENV["HOME"]` on the `ENV` object, `$stderr.puts` on an `IO`, `Time.now` on `Time`, `sleep` and `exit` and `rand` bare on `Kernel`. The C underneath each is spread through `io.c`, `file.c`, `process.c`, `time.c`, `random.c`; nothing in Ruby names the boundary.

The one thing Ruby spells `system` is `Kernel#system("ls")`, which runs a shell command. There was also a `system` gem on RubyGems.org, abandoned for over a decade.

## Portland

`System` is the floor's one face: every primitive that crosses into the host is a `System` member, and nothing else is. `System.read_file`, `System.write_file`, `System.append_file`, `System.arguments`, `System.wall_clock_time`, `System.monotonic_time`, `System.entropy`, `System.write_standard_error` are the first; the names are unabbreviated and are the floor's, not Ruby's. It is core — ships inside `pdx`, no `require`, never a tile.

Ruby's spellings are the wrappers over it, and those are what a program writes: `File.read(path)` over `System.read_file`, `Time.now` over `System.wall_clock_time`, `ARGV` over `System.arguments`, `warn` over `System.write_standard_error`. `System` is callable from any program — public — but not documented as API and not meant for application code — not published. Portland has no mechanism for that distinction; it is a social rule, as Rails's internals are, stated at the top of the `System` source itself.

`system`, lowercase, is unrelated to the module: shell execution is undecided ([#179](https://github.com/portlandlang/portland/issues/179)), and if it arrives it will be one coarse `System.spawn` with Ruby's spellings as sugar, or it will not arrive. Until then `system("ls")` refuses as any undefined name does.

## Migration

- Nothing in Ruby calls `System.anything`, so no Ruby code breaks on the module's existence — _free_.
- A program that reaches for `System.read_file` directly compiles, and a future lint can say "write `File.read`" — _gem-supplied_ in the linter's terms, since the preference is a convention.
- `system("ls")` and backticks wait on [#179](https://github.com/portlandlang/portland/issues/179); see [percent-literals.md](percent-literals.md) for `%x`.
