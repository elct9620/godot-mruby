# frozen_string_literal: true

require_relative "ruby_tests"

module Godot
  # Runs the integration-test project's checks of the loader that read
  # Godot's output: what the class index warns about, and where an error in a
  # file loaded by name is reported. Part of Godot.verify!.
  module Loader
    # What every run has to warn about, since the class index takes in every
    # game file: the files spelling names it cannot hold, and a game file
    # naming a constant mruby already has.
    WARNINGS = [
      "WARNING: res://comparable.rb names Comparable, which the realm already has, so it never loads by name",
      "WARNING: res://loader/naming/http_client.rb and res://loader/naming/httpclient.rb both name " \
      "Loader::Naming::HttpClient, so neither loads by name",
      "WARNING: res://loader/naming/crate.rb names Loader::Naming::Crate, which hides " \
      "Loader::Naming::Items::Crate from Ruby inside Loader::Naming::Items once it has loaded"
    ].freeze
    # A test using a constant whose file raises, and what its run has to print
    # in order: the error placed at the line of the file that raised.
    RAISING = "res://verify/loader/failing/raising"
    RAISED = [
      "RaisingFileTest#test_uses_a_file_that_raises [res://loader/raising.rb:12]:",
      "RuntimeError: raising.rb fails after defining its constants",
      "ERROR: RaisingFileTest#test_uses_a_file_that_raises: " \
      "RuntimeError: raising.rb fails after defining its constants",
      "(res://loader/raising.rb:12)"
    ].freeze

    module_function

    def verify!(project)
      verify_warnings!(project)
      verify_raised!(project)
    end

    # Runs the project's Ruby tests, which have to pass and warn about every
    # name the class index refused or found hidden.
    # @behavior RL-001 RL-002 RL-003
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
  end
end
