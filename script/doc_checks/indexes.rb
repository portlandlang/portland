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

# docs/ruby/ and docs/adr/ are deliberately absent: their indexes are
# generated from the files themselves, so `generated.rb` checks them exactly
# rather than merely checking that every filename appears somewhere.
#
# docs/history/ stays here because its entries carry status notes that are
# deliberately kept out of the frozen files, so nothing in a history file
# could generate them.
INDEXES = {
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
