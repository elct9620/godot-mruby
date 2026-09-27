# frozen_string_literal: true

require "json"
require "open3"

require_relative "extension"
require_relative "godot_cell"

# The third-party notices a package carries: the licenses of what the
# libraries hold, and what the extension changed in a crate it builds with.
# Backs Addon.
module Notices
  LICENSE_FILE = /\A(licen[cs]e|copying|notice)/i
  # What the extension changed in a crate, by the crate's name; MPL-2.0 asks
  # that whoever receives the library can obtain the changed source.
  CHANGES = { "godot-cell" => GodotCell.change }.freeze

  module_function

  # The licenses of what the libraries carry: mruby, linked from the archive
  # beni builds, and every crate the extension depends on at run time.
  def text
    sections = [notice("mruby", File.join(Extension::VENDOR_DIR, "mruby"), "MIT")]
    runtime_crates.each do |package|
      sections << notice("#{package["name"]} #{package["version"]}",
                         File.dirname(package["manifest_path"]), package["license"],
                         CHANGES[package["name"]])
    end

    "This addon includes the following third-party software. The source of each\n" \
      "Rust crate is available from crates.io at the listed version; a crate marked\n" \
      "changed differs from it only as its section shows.\n\n#{sections.join("\n")}"
  end

  # Some crates publish no license file and state their license only in the
  # manifest and source headers; those point at the SPDX text instead.
  def notice(title, dir, license, change = nil)
    texts = Dir.children(dir).grep(LICENSE_FILE).sort.map { |file| File.read(File.join(dir, file)) }
    if texts.empty?
      texts = ["No license file is published with this crate; the #{license} text is at https://spdx.org/licenses/\n"]
    end
    texts << change if change
    title = "#{title}, changed" if change
    "#{"=" * 78}\n#{title} (#{license || "see license files"})\n#{"=" * 78}\n\n#{texts.join("\n")}\n"
  end

  # Crates reached from the extension through normal dependencies. Build and
  # dev dependencies, and proc macros with everything under them, only run
  # while compiling and never end up in the library.
  def runtime_crates
    metadata = cargo_metadata
    packages = metadata["packages"]
    macros = packages.select { |package| proc_macro?(package) }.map { |package| package["id"] }
    ids = runtime_closure(metadata["resolve"], macros).drop(1)

    packages.select { |package| ids.include?(package["id"]) }.sort_by { |package| package["name"] }
  end

  def proc_macro?(package)
    package["targets"].any? { |target| target["kind"].include?("proc-macro") }
  end

  # Breadth-first from the root, which stays first in the result.
  def runtime_closure(resolve, excluded)
    nodes = resolve["nodes"].to_h { |node| [node["id"], node] }
    reached = [resolve["root"]]
    reached.each do |id|
      (runtime_deps(nodes.fetch(id)) - excluded).each { |dep| reached << dep unless reached.include?(dep) }
    end
    reached
  end

  def runtime_deps(node)
    node["deps"].select { |dep| dep["dep_kinds"].any? { |kind| kind["kind"].nil? } }.map { |dep| dep["pkg"] }
  end

  def cargo_metadata
    # Fetching first unpacks every crate, which is where its license files are.
    run!("cargo", "fetch", "--manifest-path", Extension::MANIFEST)
    JSON.parse(run!("cargo", "metadata", "--format-version", "1", "--manifest-path", Extension::MANIFEST))
  end

  # Finds the crates whose change the notices in `text` do not show.
  def find_missing_changes(text)
    CHANGES.reject { |crate, change| text.include?("#{crate} ") && text.include?(change) }.keys
  end

  def run!(*command)
    output, status = Open3.capture2(*command, chdir: Extension::ROOT)
    raise "#{command.first} failed: #{command.join(" ")}" unless status.success?

    output
  end
end
