#!/usr/bin/env ruby
# Benchmark the seed and the compiler on the workloads in bench/ (#25).
#
# Every workload runs both ways — direct (the seed interpreting it) and
# hosted (the seed interpreting the compiler interpreting it) — because the
# hosted/direct ratio is the number Stage 2 exists to crush. Outputs are
# compared while we're at it, so the bench doubles as a differential: a
# workload that got faster by getting wrong fails loudly.
#
# Numbers are honest, not flattering: the baseline was taken knowing it
# would be ugly, so the improvements have something to be measured against.
# Lower is better; the ratio column is the one to watch over time.
#
# `script/bench` runs the quick set (median of three runs); add `slow` to
# include the slow tier — bench/lex.pdx hosted is the compiler running its own
# front end, and it takes minutes today. That is a finding, not a bug in
# this script.
#
# A third column runs the same text under MRI, through bench/shim.rb, for
# context: how far the seed is from the Ruby it was written against. It is
# a mature bytecode VM against a tree-walking debug build, so the gap is
# large and expected. A bench Ruby cannot parse (struct patterns) or cannot
# run (the compiler itself) shows "—" there with its reason below.

require "English"

QUICK_RUNS = 3
SLOW_WORKLOADS = %w[lex].freeze
RUBY_UNRUNNABLE = { "lex" => "requires the Portland compiler" }.freeze

def repo_root = File.expand_path("..", __dir__)

def build_seed
  Dir.chdir(repo_root) do
    # rustup found the way every script finds it — see script/lib/rust.sh.
    system('sh',
           '-c',
           '. script/lib/rust.sh && cargo build --quiet --package portland-seed --bin pdx',
           exception: true)
  end
end

def timed_run(command)
  started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  output = IO.popen(command, chdir: repo_root, &:read)
  abort "bench: #{command.join(" ")} failed" unless $CHILD_STATUS.success?
  [Process.clock_gettime(Process::CLOCK_MONOTONIC) - started, output]
end

def median(durations) = durations.sort[durations.length / 2]

def measure(workload, runs, runner:)
  pdx = File.join(repo_root, "target/debug/pdx")
  command = case runner
            when :direct then [pdx, workload]
            when :hosted then [pdx, "compiler/run.pdx", workload]
            when :ruby   then ["ruby", "-r", "./bench/shim.rb", workload]
            end
  results = Array.new(runs) { timed_run(command) }
  outputs = results.map(&:last).uniq
  abort "bench: #{workload} was nondeterministic #{runner}" unless outputs.length == 1
  [median(results.map(&:first)), outputs.first]
end

# MRI's startup with the shim loaded and nothing to run: most of a small
# bench's Ruby time, so it is shown as the floor under that column.
def ruby_startup_seconds
  durations = Array.new(QUICK_RUNS) { timed_run(["ruby", "-r", "./bench/shim.rb", "-e", ""]).first }
  median(durations)
end

# Why Ruby cannot run a bench, or nil when it can: the compiler's own
# front end needs the compiler, and `struct` and `case/in` struct patterns
# are Portland's grammar, which `ruby -c` says in its own way.
def ruby_skip_reason(name, workload)
  return RUBY_UNRUNNABLE[name] if RUBY_UNRUNNABLE.key?(name)
  parses = system("ruby", "-c", workload, chdir: repo_root, out: File::NULL, err: File::NULL)
  parses ? nil : "Ruby cannot parse it"
end

include_slow = ARGV.include?("slow")
build_seed

workloads = Dir[File.join(repo_root, "bench/*.pdx")].sort
skipped = []
ruby_skipped = []
rows = []

workloads.each do |path|
  name = File.basename(path, ".pdx")
  workload = "bench/#{name}.pdx"
  if SLOW_WORKLOADS.include?(name) && !include_slow
    skipped << name
    next
  end
  runs = SLOW_WORKLOADS.include?(name) ? 1 : QUICK_RUNS
  direct_seconds, direct_output = measure(workload, runs, runner: :direct)
  hosted_seconds, hosted_output = measure(workload, runs, runner: :hosted)
  unless direct_output == hosted_output
    abort "bench: #{workload} diverged between the oracles — a bench that got faster by getting wrong:\n" \
          "direct:\n#{direct_output}hosted:\n#{hosted_output}"
  end
  ruby_reason = ruby_skip_reason(name, workload)
  ruby_seconds = nil
  if ruby_reason.nil?
    ruby_seconds, ruby_output = measure(workload, runs, runner: :ruby)
    unless ruby_output == direct_output
      abort "bench: #{workload} diverged between the seed and Ruby:\n" \
            "direct:\n#{direct_output}ruby:\n#{ruby_output}"
    end
  else
    ruby_skipped << "#{name} (#{ruby_reason})"
  end
  rows << [name, ruby_seconds, direct_seconds, hosted_seconds]
end

# MRI's own startup is most of a small bench's Ruby time — about 40ms
# against the seed's 4ms — so it is measured and shown, and the ratio is
# given to two places rather than rounded to a flattering 0x.
ruby_floor = ruby_startup_seconds

puts "| workload | ruby (s) | direct (s) | hosted (s) | direct/ruby | hosted/direct |"
puts "|----------|---------:|-----------:|-----------:|------------:|--------------:|"
rows.each do |name, ruby_seconds, direct_seconds, hosted_seconds|
  ruby_column = ruby_seconds ? format("%8.3f", ruby_seconds) : format("%8s", "—")
  seed_ratio = ruby_seconds ? format("%10.2fx", direct_seconds / ruby_seconds) : format("%11s", "—")
  puts format("| %-8s | %s | %10.3f | %10.3f | %s | %12.0fx |",
              name,
              ruby_column,
              direct_seconds,
              hosted_seconds,
              seed_ratio,
              hosted_seconds / direct_seconds)
end
puts
puts "medians of #{QUICK_RUNS} (slow tier: 1); outputs verified identical across oracles, and Ruby where it runs"
puts format("ruby's startup alone: %.3fs — the floor under every ruby (s) above", ruby_floor)
puts "skipped slow tier: #{skipped.join(", ")} — run `script/bench slow` to include" unless skipped.empty?
puts "not run under Ruby: #{ruby_skipped.join(", ")}" unless ruby_skipped.empty?
