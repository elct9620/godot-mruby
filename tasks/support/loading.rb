# frozen_string_literal: true

require_relative "ruby_tests"

module Godot
  # Runs the integration-test project's checks of loading by name: what the
  # class index warns about. Part of Godot.verify!.
  module Loading
    # A test directory whose test support spells names the class index cannot
    # hold, and what a run of it has to warn about: those, and a game file
    # naming a constant mruby already has.
    NAMING = "res://naming"
    WARNINGS = [
      "WARNING: res://comparable.rb names Comparable, which the realm already has, so it never loads by name",
      "WARNING: res://naming/http_client.rb and res://naming/httpclient.rb both name Naming::HttpClient, " \
      "so neither loads by name",
      "WARNING: res://naming/inventory.rb names Naming::Inventory, which hides Naming::Items::Inventory " \
      "from Ruby inside Naming::Items once it has loaded"
    ].freeze

    module_function

    def verify!(project)
      verify_warnings!(project)
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
  end
end
