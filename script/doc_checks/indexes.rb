# Hand-maintained indexes list every file they index.
#
# docs/adr/README.md sat at 0015 while six more ADRs shipped — four days
# was all it took. An index nobody verifies is worse than no index,
# because it reads as complete.

require_relative "lib/shared"

failures = []

def entries_under(directory)
  Dir.glob("#{REPO}/docs/#{directory}/*.md")
     .sort
     .reject { |path| File.basename(path) == "README.md" }
end

# docs/ruby/ is deliberately absent: its index is generated from each file's
# own summary line, so `generated.rb` checks it exactly rather than merely
# checking that every filename appears somewhere.
INDEXES = {
  "adr" => { entries: Dir.glob("#{REPO}/docs/adr/[0-9]*.md").sort, what: "ADRs" },
  "history" => { entries: entries_under("history"), what: "history files" }
}.freeze

INDEXES.each do |directory, index|
  index_path = "#{REPO}/docs/#{directory}/README.md"
  next unless File.exist?(index_path)

  contents = read(index_path)
  missing = index[:entries].reject { |entry| contents.include?(File.basename(entry)) }
  next if missing.empty?

  list = missing.map { |entry| "  #{File.basename(entry)}" }.join("\n")

  failures << <<~REPORT
    #{relative(index_path)}

      #{missing.length} #{index[:what]} missing from the index:

    #{list}

      Add a line for each. An index that silently stops being complete
      still reads as complete, which is how it misleads.
  REPORT
end

finish("indexes", failures, count(INDEXES.length, "index", "indexes"))
