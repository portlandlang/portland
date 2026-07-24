# Generated index sections match what the generators would produce.
#
# The hand-maintained versions of these indexes drifted the way every
# hand-maintained index does — docs/adr/README.md sat at 0015 while six more
# ADRs shipped, which took four days. Now each is built from the files it
# indexes, and this check is what makes forgetting to regenerate a failure
# rather than a slow rot.

require_relative "../lib/sections"

failures = []

SECTIONS.each do |section|
  path = "#{REPO}/#{section[:index]}"
  expected = section[:build].call
  actual = generated_section(read(path))

  if actual.nil?
    failures << <<~REPORT
      #{relative(path)}

        No generated section found. It is delimited by:

          #{GENERATED_BEGIN}
          #{GENERATED_END}

        Add those markers, then run script/docs/generate.
    REPORT
    next
  end

  next if actual == expected

  failures << <<~REPORT
    #{relative(path)}

      The generated section is out of date. Run script/docs/generate.

      #{diff_summary(actual, expected)}
  REPORT
end

finish("generated", failures, count(SECTIONS.length, "section"))
