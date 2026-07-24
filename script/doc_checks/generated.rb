# Generated index sections match what the generator would produce.
#
# The hand-maintained version of docs/ruby/'s list drifted the way every
# hand-maintained index does. Now each ledger file carries its own one-line
# summary as an italic subtitle under its H1, and the index is built from
# those — so the description lives next to the thing it describes, and
# adding a ledger file requires no edit here at all.
#
# `script/generate_docs` writes these sections; this check is what makes
# forgetting to run it a failure rather than a slow rot.

require_relative "lib/shared"
require_relative "lib/generators"

failures = []

GENERATED_SECTIONS.each do |section|
  path = "#{REPO}/#{section.fetch(:index)}"
  expected = section.fetch(:build).call
  actual = generated_section(read(path))

  if actual.nil?
    failures << <<~REPORT
      #{relative(path)}

        No generated section found. It is delimited by:

          #{GENERATED_BEGIN}
          #{GENERATED_END}

        Add those markers, then run script/generate_docs.
    REPORT
    next
  end

  next if actual == expected

  failures << <<~REPORT
    #{relative(path)}

      The generated section is out of date. Run script/generate_docs.

      #{diff_summary(actual, expected)}
  REPORT
end

finish("generated", failures, count(GENERATED_SECTIONS.length, "section"))
