# frozen_string_literal: true

require "fileutils"
require "tmpdir"

require_relative "../runner"

module Godot
  module Runner
    # Runs a copy of the integration-test project whose test settings it
    # rewrites, and reads what the runner makes of them. Part of Godot.verify!.
    module Settings
      # A test file the check writes under the copy's res://test, the pattern
      # that matches it and no other file there, and the summary of a run of it.
      PICKED = "test/picked_by_pattern.rb"
      PICKED_SOURCE = "class PickedByPatternTest < Minitest::Test\n  def test_runs\n    pass\n  end\nend\n"
      PATTERN = "picked_*.rb"
      PATTERN_SUMMARY = /^1 runs, \d+ assertions, 0 failures, 0 errors, 0 skips$/

      module_function

      # Runs a copy of the project that sets no test setting, without --dir,
      # before and after overriding its pattern.
      def verify!(project)
        Dir.mktmpdir do |dir|
          copy = File.join(dir, "project")
          FileUtils.cp_r(project, copy)
          unset_test_settings(copy)
          verify_defaults!(copy)
          verify_pattern!(copy)
        end
      end

      # A project that sets no test setting has to run the tests under
      # res://test.
      # @behavior RT-027
      def verify_defaults!(project)
        output, status = Runner.run(project)
        return if status.success? && output[Runner::PASSED, 1].to_i.positive?

        raise "A project without test settings did not run #{Runner::TESTS}:\n#{output}"
      end

      # A pattern matching one test file has to run that file's tests alone.
      # @behavior RT-028
      def verify_pattern!(project)
        File.write(File.join(project, PICKED), PICKED_SOURCE)
        File.write(File.join(project, "override.cfg"), %([mruby]\n\ntest/pattern="#{PATTERN}"\n))
        output, status = Runner.run(project)
        return if status.success? && output.match?(PATTERN_SUMMARY)

        raise "The test pattern #{PATTERN} did not narrow the run:\n#{output}"
      end

      # Removes the test settings from the project's settings, keeping the
      # others the project's files are named by.
      def unset_test_settings(project)
        path = File.join(project, "project.godot")
        File.write(path, File.read(path).gsub(%r{^test/.*\n}, ""))
      end
    end
  end
end
