# frozen_string_literal: true

require "fileutils"
require "json"
require "open3"
require "rbconfig"
require "tmpdir"

require_relative "extension"
require_relative "godot"

# Packages the addon as it is distributed and checks a package the way a
# user would install it. Backs tasks/addon.rake.
module Addon
  ROOT = Extension::ROOT
  SOURCE = File.join(ROOT, "godot", "addons", "godot_mruby")
  GDEXTENSION = File.join(SOURCE, "godot_mruby.gdextension")
  # The archive wraps addons/ in a named folder: Godot's installer before 4.7
  # strips a lone top-level folder, even when that folder is addons/.
  FOLDER = "godot-mruby"
  PACKAGE = File.join(ROOT, "pkg", "#{FOLDER}.zip")
  NOTICES = "THIRD_PARTY_LICENSES.txt"
  LICENSE_FILE = /\A(licen[cs]e|copying|notice)/i

  module_function

  # Every library godot_mruby.gdextension names; a package missing one would
  # fail to load on that platform.
  def libraries
    section = File.read(GDEXTENSION)[/^\[libraries\]\n(.*?)(?=^\[|\z)/m, 1].to_s
    section.scan(/^\s*[\w.]+\s*=\s*"([^"]+)"/).flatten.map { |path| File.join(SOURCE, path) }
  end

  def missing_libraries
    libraries.reject { |path| File.exist?(path) }
  end

  def stage(dir)
    addon = File.join(dir, FOLDER, "addons", "godot_mruby")
    FileUtils.mkdir_p(File.dirname(addon))
    FileUtils.cp_r(SOURCE, addon)
    FileUtils.cp(File.join(ROOT, "LICENSE"), addon)
    File.write(File.join(addon, NOTICES), notices)
  end

  # The licenses of what the libraries carry: mruby, linked from the archive
  # beni builds, and every crate the extension depends on at run time.
  def notices
    sections = [notice("mruby", File.join(Extension::VENDOR_DIR, "mruby"), "MIT")]
    shipped_crates.each do |package|
      sections << notice("#{package["name"]} #{package["version"]}",
                         File.dirname(package["manifest_path"]), package["license"])
    end

    "This addon includes the following third-party software. The source of each\n" \
      "Rust crate is available from crates.io at the listed version.\n\n#{sections.join("\n")}"
  end

  # Some crates publish no license file and state their license only in the
  # manifest and source headers; those point at the SPDX text instead.
  def notice(title, dir, license)
    texts = Dir.children(dir).grep(LICENSE_FILE).sort.map { |file| File.read(File.join(dir, file)) }
    if texts.empty?
      texts = ["No license file is published with this crate; the #{license} text is at https://spdx.org/licenses/\n"]
    end
    "#{"=" * 78}\n#{title} (#{license || "see license files"})\n#{"=" * 78}\n\n#{texts.join("\n")}\n"
  end

  # Crates reached from the extension through normal dependencies. Build and
  # dev dependencies, and proc macros with everything under them, only run
  # while compiling and never end up in the library.
  def shipped_crates
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

  # Windows' own tar reads zip archives; a 7z found on PATH may be the MSYS2
  # build Ruby brings, which cannot open drive-letter paths.
  def extract(package, dir)
    if RbConfig::CONFIG["host_os"].match?(/mswin|mingw/)
      FileUtils.mkdir_p(dir)
      run!(File.join(ENV.fetch("SystemRoot", "C:/Windows"), "System32", "tar.exe"), "-xf", package, "-C", dir)
    else
      run!("unzip", "-q", package, "-d", dir)
    end
  end

  # Installs the package into a copy of the test project, so what is checked
  # is the archive's content rather than the working tree.
  def verify!(package)
    Dir.mktmpdir do |dir|
      project = File.join(dir, "project")
      copy_project(project)
      extract(package, File.join(dir, "package"))
      FileUtils.mv(File.join(dir, "package", FOLDER, "addons", "godot_mruby"), File.join(project, "addons"))
      Godot.verify!(project)
    end
  end

  # Of .godot, the copy keeps only what a fresh checkout has: the extension list.
  def copy_project(project)
    FileUtils.cp_r(Godot::PROJECT, project)
    FileUtils.rm_rf(File.join(project, ".godot"))
    FileUtils.mkdir_p(File.join(project, ".godot"))
    FileUtils.cp(File.join(Godot::PROJECT, Godot::EXTENSION_LIST), File.join(project, Godot::EXTENSION_LIST))
    FileUtils.rm_rf(File.join(project, "addons", "godot_mruby"))
    FileUtils.mkdir_p(File.join(project, "addons"))
  end

  def run!(*command)
    output, status = Open3.capture2(*command, chdir: ROOT)
    raise "#{command.first} failed: #{command.join(" ")}" unless status.success?

    output
  end
end
