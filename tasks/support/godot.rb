# frozen_string_literal: true

require "open3"

# Runs the integration-test project headless and reads what it reports.
# Backs tasks/godot.rake.
module Godot
  PROJECT = File.expand_path("../../godot", __dir__)
  EXECUTABLE = ENV.fetch("GODOT", "godot")
  # What gdext prints once the engine has called into the library.
  LOADED = "Initialize godot-rust"
  FAILED = /^(ERROR|SCRIPT ERROR):/

  module_function

  # An editor pass is what records the addon's .gdextension for later runs and
  # loads it; the project has no main scene for a plain run to start. Godot
  # reports a library it could not open on stdout and may still exit 0, so the
  # pass is judged by what it printed.
  def verify_loaded!(project = PROJECT)
    output, status = Open3.capture2e(EXECUTABLE, "--headless", "--editor", "--quit", "--path", project)
    errors = output.lines.grep(FAILED)
    return if status.success? && errors.empty? && output.include?(LOADED)

    raise "The extension did not load:\n#{errors.empty? ? output : errors.join}"
  end
end
