# frozen_string_literal: true

require "fileutils"
require "open3"
require "rbconfig"
require "tmpdir"

require_relative "extension"
require_relative "godot"
require_relative "notices"
require_relative "release"

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

  module_function

  # Every library godot_mruby.gdextension names; a package missing one would
  # fail to load on that platform.
  def libraries
    section = File.read(GDEXTENSION)[/^\[libraries\]\n(.*?)(?=^\[|\z)/m, 1].to_s
    section.scan(/^\s*[\w.]+\s*=\s*"([^"]+)"/).flatten.map { |path| File.join(SOURCE, path) }
  end

  def absent_libraries
    libraries.reject { |path| File.exist?(path) }
  end

  def stage(dir)
    addon = File.join(dir, FOLDER, "addons", "godot_mruby")
    FileUtils.mkdir_p(File.dirname(addon))
    FileUtils.cp_r(SOURCE, addon)
    FileUtils.cp(File.join(ROOT, "LICENSE"), addon)
    File.write(File.join(addon, NOTICES), Notices.text)
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
      project = install(package, dir)
      Godot.verify!(project)
    end
  end

  # Plays the release checks with the package installed, so what a shipped
  # game runs is the archive's library.
  def verify_release!(package)
    Dir.mktmpdir do |dir|
      Godot::Release.verify!(install(package, dir))
    end
  end

  # A copy of the test project in `dir` with the package installed, once its
  # notices are checked.
  def install(package, dir)
    project = File.join(dir, "project")
    copy_project(project)
    extract(package, File.join(dir, "package"))
    verify_notices!(File.join(dir, "package", FOLDER, "addons", "godot_mruby", NOTICES))
    FileUtils.mv(File.join(dir, "package", FOLDER, "addons", "godot_mruby"), File.join(project, "addons"))
    project
  end

  # @behavior RA-003
  def verify_notices!(path)
    missing = Notices.find_missing_changes(File.read(path))
    raise "#{NOTICES} does not show the change to #{missing.join(", ")}" unless missing.empty?
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
