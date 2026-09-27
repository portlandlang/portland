# spec/ruby_spec.yml says which ruby/spec files the suite has taken in; the
# dashboard at portlandlang.com is drawn from it. Everything here is checkable
# without a ruby/spec checkout — the upstream paths are checked by the
# generator, which has one — so the gate can hold the manifest's shape:
#
# - every status is one a human may write (failing and missing are computed)
# - an imported entry names pdx specs that exist; a skip names none
# - a partial or a skip says why, and any link it gives resolves
# - a directory key is a skip, since a directory is never imported whole

require "yaml"
require_relative "../lib/shared"

MANIFEST = "#{REPO}/spec/ruby_spec.yml"
WRITABLE = %w[partial_planned partial_intentional skipped].freeze

failures = []
manifest = YAML.safe_load_file(MANIFEST)

manifest.each do |key, entry|
  entry ||= {}
  status = entry["status"]
  pdx = Array(entry["pdx"])
  where = "spec/ruby_spec.yml: #{key}"

  failures << "#{where}\n\n  status #{status.inspect} is not one a human writes — use #{WRITABLE.join(', ')}, or none for passing.\n" if status && !WRITABLE.include?(status)
  failures << "#{where}\n\n  a directory key covers a whole directory, which only a skip does.\n" if key.end_with?("/") && status != "skipped"
  failures << "#{where}\n\n  an imported entry names the pdx specs that hold it.\n" if status != "skipped" && pdx.empty?
  failures << "#{where}\n\n  a skip imports nothing, so it names no pdx specs.\n" if status == "skipped" && pdx.any?
  failures << "#{where}\n\n  a partial or a skip says why, in `reason`.\n" if status && entry["reason"].to_s.strip.empty?

  pdx.each do |file|
    failures << "#{where}\n\n  #{file} does not exist.\n" unless File.exist?("#{REPO}/#{file}")
  end

  link = entry["link"]
  next if link.nil? || link.start_with?("https://")

  failures << "#{where}\n\n  link #{link} does not exist in this repo.\n" unless File.exist?("#{REPO}/#{link}")
end

finish("ruby_spec_manifest", failures, count(manifest.size, "entry", "entries"))
