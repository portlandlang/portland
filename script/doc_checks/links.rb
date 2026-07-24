# Internal doc links resolve.

require_relative "shared"

failures = []

markdown_files.each do |path|
  read_lines(path).each_with_index do |line, index|
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

finish("links", failures, "#{markdown_files.length} files")
