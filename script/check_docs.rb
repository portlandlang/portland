# Mechanical checks over the docs. Each exists because a real mistake got
# shipped and was caught by a human reading, not by a tool.
#
# Only the mechanically-checkable class lives here. Judgment errors — a
# prediction used as a reason, a bullet that flattens distinct cases — are
# not detectable this way and stay the author's job.
#
# A check that cries wolf is worse than no check, so each one declines
# where it cannot tell (AGENT.md: never guess, in the implementation too).

REPO = File.expand_path("..", __dir__)

def markdown_files
  Dir.glob("#{REPO}/**/*.md").reject do |path|
    path.include?("/target/") || path.include?("/_scratch/")
  end
end

def relative(path) = path.delete_prefix("#{REPO}/")

# Lines inside ``` fences, as [line, one_indexed_number] pairs.
#
# Prose *discusses* wrong forms ("`Foo::bar()` is an error"); code blocks
# *demonstrate* usage. Only the second is checkable without guessing.
def fenced_lines(path)
  inside = false
  File.readlines(path).each_with_index.filter_map do |line, index|
    if line.lstrip.start_with?("```")
      inside = !inside
      next
    end
    [line, index + 1] if inside
  end
end

failures = []

# 1. `::` names, `.` invokes (ADR 0021).
#
# A draft once wrote `Statistics::mean(data)` two messages after declaring
# exactly that an error. In a code block, a path segment followed by `(`
# is an invocation written with the naming operator.
markdown_files.each do |path|
  fenced_lines(path).each do |line, number|
    # A block may deliberately show the wrong form to name it as wrong.
    next if line.match?(/#.*(error|wrong|instead)/i)

    found = line.match(/\b([A-Z]\w*(?:::\w+)*)::([a-z_]\w*)\(/)
    next unless found

    namespace, method = found.captures
    failures << <<~REPORT
      #{relative(path)}:#{number}
        #{line.strip}

        `::` names, `.` invokes (ADR 0021). This calls #{method}, so it needs
        a dot — write #{namespace}.#{method}(...) instead.
    REPORT
  end
end

# 2. Every ADR is reflected in the Ruby ledger, or says why not.
#
# Splats were once documented nowhere: ADR 0014 matched Ruby so closely
# that nobody noticed it *also* deferred something.
ledger = Dir.glob("#{REPO}/docs/ruby/*.md").map { |path| File.read(path) }.join
changelog = File.read("#{REPO}/CHANGELOG.md")

Dir.glob("#{REPO}/docs/adr/[0-9]*.md").sort.each do |path|
  number = File.basename(path)[0, 4]
  next if ledger.match?(/adr\/#{number}-/) || ledger.match?(/ADR #{number}\b/)
  # An ADR may opt out by saying, in itself or the changelog, that it is a
  # deliberate non-difference from Ruby.
  next if File.read(path).match?(/non-difference/i)
  next if changelog.match?(/ADR #{number}\b[^\n]*non-difference/i)

  failures << <<~REPORT
    #{relative(path)}

      No entry in docs/ruby/ cites this ADR, so a migrating Rubyist has
      nowhere to read what changed. Either add a ledger file linking
      ../adr/#{number}-…, or — if this decision matches Ruby exactly —
      say "non-difference" in the ADR to opt out.
  REPORT
end

# 3. Internal doc links resolve.
markdown_files.each do |path|
  File.readlines(path).each_with_index do |line, index|
    line.scan(/\]\(([^)#][^)]*)\)/) do |target,|
      next if target.start_with?("http")

      resolved = File.expand_path(target.split("#").first, File.dirname(path))
      next if File.exist?(resolved)

      failures << <<~REPORT
        #{relative(path)}:#{index + 1}

          Link to #{target} goes nowhere — expected a file at
          #{relative(resolved)}. Fix the path, or create the file.
      REPORT
    end
  end
end

# 4. Every current doc names its reader (docs/principles.md, #11).
#
# A doc written for everyone is written for nobody — AGENT.md spent two
# weeks as the de-facto source of truth because nothing said it was for
# the agent, so README was allowed to fall behind it.
#
# Exempt, deliberately: README.md (the front door's reader is "whoever
# arrives"), CHANGELOG.md (a log), individual ADRs and ledger files (they
# carry a Status line instead), and frozen history files (their folder's
# README carries the contract for all of them).
audience_required =
  ["#{REPO}/AGENT.md", "#{REPO}/ROADMAP.md"] +
  Dir.glob("#{REPO}/docs/*.md") +
  Dir.glob("#{REPO}/docs/*/README.md")

audience_required.sort.each do |path|
  # The line may wrap, so only its opening is anchored.
  next if File.readlines(path).first(10).any? { |line| line.start_with?("_For: ") }

  failures << <<~REPORT
    #{relative(path)}

      No reader named. Add a line near the top like:

        _For: anyone deciding whether to try Portland._

      A doc written for everyone is written for nobody, and the ones
      without a stated reader are the ones that drift into summarizing
      every other doc.
  REPORT
end

# 5. Hand-maintained indexes list every file they index.
#
# docs/adr/README.md sat at 0015 while six more ADRs shipped — four days
# was all it took. An index nobody verifies is worse than no index,
# because it reads as complete.
def check_index(index_path, entries, failures, what)
  return unless File.exist?(index_path)

  index = File.read(index_path)
  missing = entries.reject { |entry| index.include?(File.basename(entry)) }
  return if missing.empty?

  list = missing.map { |entry| "  #{File.basename(entry)}" }.join("\n")

  failures << <<~REPORT
    #{relative(index_path)}

      #{missing.length} #{what} missing from the index:

    #{list}

      Add a line for each. An index that silently stops being complete
      still reads as complete, which is how it misleads.
  REPORT
end

check_index(
  "#{REPO}/docs/adr/README.md",
  Dir.glob("#{REPO}/docs/adr/[0-9]*.md").sort,
  failures,
  "ADRs"
)

check_index(
  "#{REPO}/docs/ruby/README.md",
  Dir.glob("#{REPO}/docs/ruby/*.md").sort.reject { |path| File.basename(path) == "README.md" },
  failures,
  "ledger files"
)

check_index(
  "#{REPO}/docs/history/README.md",
  Dir.glob("#{REPO}/docs/history/*.md").sort.reject { |path| File.basename(path) == "README.md" },
  failures,
  "history files"
)

# Deliberately absent: a check that doc code samples run.
#
# It would have caught three constructs invented for docs/language.md in
# one sitting (endless methods, a one-line if/then/else, a ternary), so it
# is worth having — but `pdx` has no parse-only mode, and telling a parse
# failure from a runtime one by matching panic text would be a string
# match on interpreter internals. That is precisely the fragility the REPL
# already suffers from. It needs a `pdx --parse` flag first — issue #35.

# Deliberately absent: a CHANGELOG newest-first check. The ordering is
# chronological, and nothing in an entry's *content* reveals its date —
# entries cite several ADRs or none, so ADR numbers do not descend even
# when the order is correct. Rather than ship a check that fires on
# correct files, the rule stays in AGENT.md for a human to hold: insert
# directly under the `## Unreleased` header, never relative to a
# neighbouring bullet.

if failures.empty?
  puts "docs ok — #{markdown_files.length} files checked"
  exit 0
end

warn "docs check failed:\n\n#{failures.join("\n")}\n"
exit 1
