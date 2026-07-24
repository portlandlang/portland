# Every ADR is reflected in the Ruby ledger, or says why not.
#
# Splats were once documented nowhere: ADR 0014 matched Ruby so closely
# that nobody noticed it *also* deferred something.

require_relative "shared"

failures = []

ledger = Dir.glob("#{REPO}/docs/ruby/*.md").map { |path| read(path) }.join
changelog = read("#{REPO}/CHANGELOG.md")
adrs = Dir.glob("#{REPO}/docs/adr/[0-9]*.md").sort

adrs.each do |path|
  number = File.basename(path)[0, 4]
  next if ledger.match?(/adr\/#{number}-/) || ledger.match?(/ADR #{number}\b/)
  # An ADR may opt out by saying, in itself or the changelog, that it is a
  # deliberate non-difference from Ruby.
  next if read(path).match?(/non-difference/i)
  next if changelog.match?(/ADR #{number}\b[^\n]*non-difference/i)

  failures << <<~REPORT
    #{relative(path)}

      No entry in docs/ruby/ cites this ADR, so a migrating Rubyist has
      nowhere to read what changed. Either add a ledger file linking
      ../adr/#{number}-…, or — if this decision matches Ruby exactly —
      say "non-difference" in the ADR to opt out.
  REPORT
end

finish("ledger_coverage", failures, "#{adrs.length} ADRs")
