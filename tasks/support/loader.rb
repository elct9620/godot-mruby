# frozen_string_literal: true

require_relative "ruby_tests"

module Godot
  # Runs the integration-test project's checks of the loader that read
  # the log or a run's outcome: what the class index warns about, where
  # an error in a file loaded by name is reported, and that a test class
  # loaded during a run does not run. Part of Godot.verify!.
  module Loader
    # What every run has to warn about, since the class index takes in every
    # game file: the files spelling names it cannot hold, and game files
    # naming a constant mruby has, the realm opens with, or the runner installs.
    WARNINGS = [
      "WARNING: res://comparable.rb names Comparable, which the realm already has, so it never loads by name",
      "WARNING: res://godot.rb names Godot, which the realm already has, so it never loads by name",
      "WARNING: res://minitest.rb names Minitest, which the realm already has, so it never loads by name",
      "WARNING: res://test/unit/loader/naming/http_client.rb and res://test/unit/loader/naming/httpclient.rb " \
      "both name Unit::Loader::Naming::HttpClient, so neither loads by name",
      "WARNING: res://test/unit/loader/naming/crate.rb names Unit::Loader::Naming::Crate, which hides " \
      "Unit::Loader::Naming::Items::Crate from Ruby inside Unit::Loader::Naming::Items once it has loaded"
    ].freeze
    # A test using a constant whose file raises, and what its run has to print
    # in order: the error placed at the line of the file that raised.
    RAISING = "res://verify/loader/failing/raising"
    RAISED = [
      "RaisingFileTest#test_uses_a_file_that_raises [res://test/unit/loader/raising.rb:13]:",
      "RuntimeError: raising.rb fails after defining its constants",
      "ERROR: RaisingFileTest#test_uses_a_file_that_raises: " \
      "RuntimeError: raising.rb fails after defining its constants",
      "(res://test/unit/loader/raising.rb:13)"
    ].freeze
    # A test directory whose test loads a test class by name during the run,
    # and the summary that shows the loaded class's test did not run.
    LATE = "res://verify/loader/late"
    LATE_SUMMARY = /^1 runs, \d+ assertions, 0 failures, 0 errors, 0 skips$/

    module_function

    def verify!(project)
      verify_warnings!(project)
      verify_raised!(project)
      verify_late!(project)
    end

    # Runs the project's Ruby tests, which have to pass and warn about every
    # name the class index refused or found hidden.
    # @behavior RL-001 RL-002 RL-003 RL-024 RL-025
    def verify_warnings!(project)
      output, status = RubyTests.run(project, "--dir", RubyTests::TESTS)
      missing = WARNINGS.reject { |warning| output.include?(warning) }
      return if status.success? && missing.empty?

      raise "A run of #{RubyTests::TESTS} did not warn as it should #{missing}:\n#{output}"
    end

    # Runs the test whose constant's file raises, which has to fail with the
    # error at the file's line.
    # @behavior RL-021
    def verify_raised!(project)
      output, status = RubyTests.run(project, "--dir", RAISING)
      missing = RubyTests.missing_in_order(output, RAISED)
      return if status.exitstatus == 1 && missing.empty?

      raise "A run of #{RAISING} did not fail as it should #{missing}:\n#{output}"
    end

    # Runs the test that loads a test class by name, which has to pass with its
    # own test alone.
    # @behavior RL-013
    def verify_late!(project)
      output, status = RubyTests.run(project, "--dir", LATE)
      return if status.success? && output.match?(LATE_SUMMARY)

      raise "A run of #{LATE} ran a test class loaded by name:\n#{output}"
    end
  end
end
