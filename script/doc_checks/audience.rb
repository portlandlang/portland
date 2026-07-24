# Every current doc names its reader (docs/principles.md, #11).
#
# A doc written for everyone is written for nobody — AGENT.md spent two
# weeks as the de-facto source of truth because nothing said it was for
# the agent, so README was allowed to fall behind it.
#
# Exempt, deliberately: README.md (the front door's reader is "whoever
# arrives"), CHANGELOG.md (a log), individual ADRs and ledger files (they
# carry a Status line instead), and frozen history files (their folder's
# README carries the contract for all of them).

require_relative "lib/shared"

failures = []

required =
  ["#{REPO}/AGENT.md", "#{REPO}/ROADMAP.md"] +
  Dir.glob("#{REPO}/docs/*.md") +
  Dir.glob("#{REPO}/docs/*/README.md")

required.sort.each do |path|
  # The line may wrap, so only its opening is anchored.
  next if read_lines(path).first(10).any? { |line| line.start_with?("_For: ") }

  failures << <<~REPORT
    #{relative(path)}

      No reader named. Add a line near the top like:

        _For: anyone deciding whether to try Portland._

      A doc written for everyone is written for nobody, and the ones
      without a stated reader are the ones that drift into summarizing
      every other doc.
  REPORT
end

finish("audience", failures, count(required.length, "doc"))
