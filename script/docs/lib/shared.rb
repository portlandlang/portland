# Helpers shared by every check in ../checks/ and every generator in
# ../generators/.
#
# Each is its own file, so adding one means adding a file rather than editing
# a growing script, and so one can be run alone while you iterate:
# `script/docs/check links`, `script/docs/generate adr`.
#
# These are plain `.rb` libraries, not executables — no shebang, no exec bit —
# because the wrapper scripts are what select a Ruby 3+, and macOS's system
# Ruby is 2.6 and cannot parse this file. A shebang would advertise a way to
# run them that quietly picks the wrong interpreter.
#
# Adding a check: drop `something.rb` in ../checks/, require this file, and
# end with `finish`. Adding a generator: drop `something.rb` in
# ../generators/ and call `generates`. Neither needs wiring anywhere.

REPO = File.expand_path("../../..", __dir__)

def relative(path) = path.delete_prefix("#{REPO}/")

# The docs are UTF-8; a caller's locale is not our business. Ruby reads files
# as Encoding.default_external, which is US-ASCII whenever LANG is unset — a
# git hook, a bare CI shell — and then the first em dash in our own prose
# raises `invalid byte sequence` before any check can run. (Source encoding
# is UTF-8 regardless since Ruby 2.0; this is the external half.)
def read(path) = File.read(path, encoding: "UTF-8")

def read_lines(path) = File.readlines(path, encoding: "UTF-8")

def markdown_files
  Dir.glob("#{REPO}/**/*.md").reject do |path|
    path.include?("/target/") || path.include?("/_scratch/")
  end
end

# Lines inside ``` fences, as [line, one_indexed_number] pairs.
#
# Prose *discusses* wrong forms ("`Foo::bar()` is an error"); code blocks
# *demonstrate* usage. Only the second is checkable without guessing.
def fenced_lines(path)
  inside = false
  read_lines(path).each_with_index.filter_map do |line, index|
    if line.lstrip.start_with?("```")
      inside = !inside
      next
    end
    [line, index + 1] if inside
  end
end

# "1 file", "51 files", "2 indexes" — a summary that says "1 sections" reads
# like a bug in the checker, which undermines the checker.
def count(number, singular, plural = "#{singular}s")
  "#{number} #{number == 1 ? singular : plural}"
end

# Every check ends the same way: say what passed, or report and fail.
def finish(name, failures, summary)
  if failures.empty?
    puts "  ok    #{name} — #{summary}"
    exit 0
  end

  warn "  FAIL  #{name}\n\n#{failures.join("\n")}"
  exit 1
end

# --- Generated sections ------------------------------------------------
#
# An index whose content is entirely derivable from the files it indexes is
# generated; anything editorial stays hand-written, because a generator
# flattens judgment into whatever field it reads from.

GENERATED_BEGIN = "<!-- generated: do not edit by hand — script/docs/generate -->"
GENERATED_END = "<!-- /generated -->"

# Each file in ../generators/ calls this once. The registry is what lets both
# `script/docs/generate` (which writes) and checks/generated.rb (which
# verifies) agree about what "current" means without either knowing the list.
#
# The name is given rather than derived from the index path: every index is a
# README.md, so deriving it named all three generators "README".
SECTIONS = []

def generates(name, index:, &build)
  SECTIONS << { name: name, index: index, build: build }
end

def heading(path)
  found = read_lines(path).first
  return nil unless found&.start_with?("# ")

  found.delete_prefix("# ").strip
end

# The italic line under a doc's H1 — the same shape as the `_For: …_` audience
# line, readable in the file itself rather than metadata that only an index
# consumes.
def subtitle(path)
  found = read_lines(path)[1..4].to_a.find { |line| line.start_with?("_") && line.strip.end_with?("_") }
  return nil unless found

  found.strip.delete_prefix("_").delete_suffix("_")
end

# Title — summary, for a directory whose files each carry their own summary.
def summary_index(directory)
  Dir.glob("#{REPO}/docs/#{directory}/*.md")
     .reject { |path| File.basename(path) == "README.md" }
     .sort
     .map do |path|
       title = heading(path) or abort "#{relative(path)}: no `# ` heading to index"
       line = subtitle(path) or abort "#{relative(path)}: no italic summary line under the heading"

       "- [#{title}](#{File.basename(path)}) — #{line}"
     end.join("\n")
end

# The text between the markers, or nil when they are absent.
def generated_section(contents)
  first = contents.index(GENERATED_BEGIN)
  last = contents.index(GENERATED_END)
  return nil if first.nil? || last.nil?

  contents[(first + GENERATED_BEGIN.length)...last].strip
end

def replace_generated_section(contents, replacement)
  first = contents.index(GENERATED_BEGIN)
  last = contents.index(GENERATED_END)
  return nil if first.nil? || last.nil?

  "#{contents[0...(first + GENERATED_BEGIN.length)]}\n\n#{replacement}\n\n#{contents[last..]}"
end

def diff_summary(actual, expected)
  added = expected.lines.map(&:chomp) - actual.lines.map(&:chomp)
  removed = actual.lines.map(&:chomp) - expected.lines.map(&:chomp)

  summary = added.map { |line| "  + #{line}" } + removed.map { |line| "  - #{line}" }
  summary << "  (same lines, different order)" if summary.empty?
  summary.join("\n")
end
