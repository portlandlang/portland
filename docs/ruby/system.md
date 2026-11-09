# `System`

**Summary:** Ruby has no module for the host boundary — its syscall-touching functions are scattered across `Kernel`, `File`, `Process`, `ENV`, `IO`, and `Time`; Portland gathers every host-touching primitive under `System`, public but not published, and keeps Ruby's spellings on the wrappers written over it.

**Status:** decided ([ADR 0068](../adr/0068-2026-10-25-system-is-the-floor-module.md), 2026-10-25); the first wave — `arguments`, `read_file`, `write_file`, `append_file`, `write_standard_output`, `write_standard_error`, `wall_clock_time`, `monotonic_time`, `entropy` — built on both implementations 2026-10-25/26 with the three bare placeholders retired ([#176](https://github.com/portlandlang/portland/issues/176)).

## Ruby

There is no `System` constant. The functions that touch the operating system live wherever their receiver suggests: `File.read` and `File.write` on `File`, `Process.pid` and `Process.clock_gettime` on `Process`, `ENV["HOME"]` on the `ENV` object, `$stderr.puts` on an `IO`, `Time.now` on `Time`, `sleep` and `exit` and `rand` bare on `Kernel`. The C underneath each is spread through `io.c`, `file.c`, `process.c`, `time.c`, `random.c`; nothing in Ruby names the boundary.

The one thing Ruby spells `system` is `Kernel#system("ls")`, which runs a shell command. There was also a `system` gem on RubyGems.org, abandoned for over a decade.

## Portland

`System` is the floor's one face: every primitive that crosses into the host is a `System` member, and nothing else is. `System.read_file`, `System.write_file`, `System.append_file`, `System.arguments`, `System.wall_clock_time`, `System.monotonic_time`, `System.entropy`, `System.write_standard_error` are the first; the names are unabbreviated and are the floor's, not Ruby's. It is core — ships inside `pdx`, no `require`, never a tile.

Ruby's spellings are the wrappers over it, and those are what a program writes: `File.read(path)` over `System.read_file`, `Time.now` over `System.wall_clock_time`, `ARGV` over `System.arguments`, `warn` over `System.write_standard_error` — and `puts`, `p`, `pp`, `print`, `putc`, and `printf` over `System.write_standard_output`, each answering what Ruby's does: `p` its argument, several as an array, nil with none; `putc` its argument; `print`, `printf`, and `warn` nil; `puts` no value at all ([#185](https://github.com/portlandlang/portland/issues/185)). `System` is callable from any program — public — but not documented as API and not meant for application code — not published. Portland has no mechanism for that distinction; it is a social rule, as Rails's internals are, stated at the top of the `System` source itself.

Where a test in Ruby swaps `$stdout` for a `StringIO` to see what a call wrote, as mspec's `output` matcher and RSpec's `output(...).to_stdout` do, Portland has `System.capture_standard_output { … }` and `System.capture_standard_error { … }` ([ADR 0068](../adr/0068-2026-10-25-system-is-the-floor-module.md), amended, [#251](https://github.com/portlandlang/portland/issues/251)): each runs its block and answers the text written to that stream, in place of writing it. Portland has no `$stdout` to reassign, so the capture is the one way to redirect a stream. It covers the interpreter's writes, not the file descriptor, so what mspec's `output_to_fd` checks (a write straight to fd 1, or by a child process) is not captured.

`system`, lowercase, is unrelated to the module: shell execution is undecided ([#179](https://github.com/portlandlang/portland/issues/179)), and if it arrives it will be one coarse `System.spawn` with Ruby's spellings as sugar, or it will not arrive. Until then `system("ls")` refuses as any undefined name does.

## Migration

- Nothing in Ruby calls `System.anything`, so no Ruby code breaks on the module's existence — _free_.
- A program that reaches for `System.read_file` directly compiles, and a future lint can say "write `File.read`" — _gem-supplied_ in the linter's terms, since the preference is a convention.
- A test that reassigns `$stdout` or `$stderr` to read back a write becomes `System.capture_standard_output { … }` or `System.capture_standard_error { … }`, which answers the text: `expect(System.capture_standard_output { print "x" }).to eq("x")` — _gem-supplied_, since the call is valid Ruby and a gem can define `System.capture_standard_output` over a `$stdout` swap. A test that reopens the file descriptor has no rewrite yet.
- `system("ls")` and backticks wait on [#179](https://github.com/portlandlang/portland/issues/179); see [percent-literals.md](percent-literals.md) for `%x`.
