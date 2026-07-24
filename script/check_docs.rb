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
