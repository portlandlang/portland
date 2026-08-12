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

require "English"

QUICK_RUNS = 3
SLOW_WORKLOADS = %w[lex].freeze

def repo_root = File.expand_path("..", __dir__)

def build_seed
  Dir.chdir(repo_root) do
    system({ "PATH" => "/opt/homebrew/opt/rustup/bin:#{ENV.fetch("PATH")}" },
           "cargo", "build", "--quiet", "--package", "portland-seed", "--bin", "pdx",
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

def measure(workload, runs, hosted:)
  pdx = File.join(repo_root, "target/debug/pdx")
  command = hosted ? [pdx, "compiler/run.pdx", workload] : [pdx, workload]
  results = Array.new(runs) { timed_run(command) }
  outputs = results.map(&:last).uniq
  abort "bench: #{workload} was nondeterministic #{hosted ? "hosted" : "direct"}" unless outputs.length == 1
  [median(results.map(&:first)), outputs.first]
end

include_slow = ARGV.include?("slow")
build_seed

workloads = Dir[File.join(repo_root, "bench/*.pdx")].sort
skipped = []
rows = []

workloads.each do |path|
  name = File.basename(path, ".pdx")
  workload = "bench/#{name}.pdx"
  if SLOW_WORKLOADS.include?(name) && !include_slow
    skipped << name
    next
  end
  runs = SLOW_WORKLOADS.include?(name) ? 1 : QUICK_RUNS
  direct_seconds, direct_output = measure(workload, runs, hosted: false)
  hosted_seconds, hosted_output = measure(workload, runs, hosted: true)
  unless direct_output == hosted_output
    abort "bench: #{workload} diverged between the oracles — a bench that got faster by getting wrong:\n" \
          "direct:\n#{direct_output}hosted:\n#{hosted_output}"
  end
  rows << [name, direct_seconds, hosted_seconds, hosted_seconds / direct_seconds]
end

puts "| workload | direct (s) | hosted (s) | hosted/direct |"
puts "|----------|-----------:|-----------:|--------------:|"
rows.each do |name, direct_seconds, hosted_seconds, ratio|
  puts format("| %-8s | %10.3f | %10.3f | %12.0fx |", name, direct_seconds, hosted_seconds, ratio)
end
puts
puts "medians of #{QUICK_RUNS} (slow tier: 1); outputs verified identical across oracles"
puts "skipped slow tier: #{skipped.join(", ")} — run `script/bench slow` to include" unless skipped.empty?
