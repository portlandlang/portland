# Loads every generator, so SECTIONS is populated.
#
# Required by both `script/docs/generate` (which writes the sections) and
# checks/generated.rb (which verifies them), so neither holds a list that
# could fall out of step with what is on disk.

require_relative "shared"

Dir.glob("#{__dir__}/../generators/*.rb").sort.each { require_relative it }
