# frozen_string_literal: true

require_relative "ruby_tests"

module Godot
  # Runs the integration-test project's checks of loading by name that read
  # Godot's output: what the class index warns about, and where an error in a
  # file loaded by name is reported. Part of Godot.verify!.
  module Loading
    # A test directory whose test support spells names the class index cannot
    # hold, and what a run of it has to warn about: those, and a game file
    # naming a constant mruby already has.
    NAMING = "res://naming"
    WARNINGS = [
      "WARNING: res://comparable.rb names Comparable, which the realm already has, so it never loads by name",
      "WARNING: res://naming/http_client.rb and res://naming/httpclient.rb both name Naming::HttpClient, " \
      "so neither loads by name",
      "WARNING: res://naming/crate.rb names Naming::Crate, which hides Naming::Items::Crate " \
      "from Ruby inside Naming::Items once it has loaded"
    ].freeze
    # A test using a constant whose file raises, and what its run has to print
    # in order: the error placed at the line of the file that raised.
    RAISING = "res://failing/loading"
    RAISED = [
      "RaisingTest#test_uses_a_file_that_raises [res://loading/raising.rb:12]:",
      "RuntimeError: raising.rb fails after defining its constants",
      "ERROR: RaisingTest#test_uses_a_file_that_raises: RuntimeError: raising.rb fails after defining its constants",
      "(res://loading/raising.rb:12)"
    ].freeze

    module_function

    def verify!(project)
      verify_warnings!(project)
      verify_raised!(project)
    end

    # Runs the naming directory, which has to pass and warn about every name
    # its class index refused or found hidden.
    # @behavior RL-001 RL-002 RL-003
    def verify_warnings!(project)
      output, status = RubyTests.run(project, "--dir", NAMING)
      missing = WARNINGS.reject { |warning| output.include?(warning) }
      return if status.success? && missing.empty?

      raise "A run of #{NAMING} did not warn as it should #{missing}:\n#{output}"
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
