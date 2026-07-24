# Helpers shared by the checks in this directory.
#
# Each check is its own file, so adding one means adding a file rather than
# editing a growing script, and so one can be run alone while you iterate:
# `script/check_docs links`. `script/check_docs` with no arguments runs all
# of them.
#
# These are plain `.rb` libraries, not executables — no shebang, no exec
# bit — because the runner is what selects a Ruby 3+, and macOS's system
# Ruby is 2.6 and cannot parse this file. A shebang would advertise a way
# to run them that quietly picks the wrong interpreter.
#
# Adding a check: drop `something.rb` in here, require this file, and end
# with `finish`. The runner picks it up with no wiring.

REPO = File.expand_path("../..", __dir__)

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

# Every check ends the same way: say what passed, or report and fail.
def finish(name, failures, summary)
  if failures.empty?
    puts "  ok    #{name} — #{summary}"
    exit 0
  end

  warn "  FAIL  #{name}\n\n#{failures.join("\n")}"
  exit 1
end
