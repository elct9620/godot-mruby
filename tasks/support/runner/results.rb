# frozen_string_literal: true

require "fileutils"
require "json"
require "open3"
require "tmpdir"

require_relative "../runner"

module Godot
  module Runner
    # Makes runs that write their results with --results, as the editor's test
    # panel reads them, and a run that takes its options from the run file the
    # panel leaves. Part of Godot.verify!.
    module Results
      # Directories of tests that pass, skip and fail, each with the tests its
      # results have to list, and where the failure has to be placed.
      RUNS = {
        "#{RUNNER_TESTS}/skip" => [%w[SkipTest test_passes pass], %w[SkipTest test_skips skip]],
        "#{RUNNER_TESTS}/failing/assertion" => [%w[FailedAssertionTest test_one_equals_two failure]]
      }.freeze
      FAILED_AT = ["#{RUNNER_TESTS}/failing/assertion/failed_assertion_test.rb", 9].freeze
      # A directory whose test file does not parse, and the line it fails at.
      UNPARSED = "#{RUNNER_TESTS}/failing/syntax".freeze
      UNPARSED_AT = ["#{UNPARSED}/broken_syntax_test.rb", 4].freeze
      # The script that prints where user:// is, and the run file under it.
      USER_DIR_SCRIPT = "res://integration/runner/run_file/user_dir.gd"
      USER_DIR = /^user_dir=(.+)$/
      RUN_FILE = File.join("godot_mruby", "run.json")

      module_function

      def verify!(project)
        verify_listed!(project)
        verify_load_failure!(project)
        verify_run_file!(project)
      end

      # Each directory's results list its tests by class and name with their
      # results, and the failure at its Ruby line.
      # @behavior RT-036
      def verify_listed!(project)
        RUNS.each do |directory, expected|
          tests = results_of(project, "--dir", directory)["tests"]
          listed = tests.map { |test| test.values_at("class", "name", "result") }
          raise "The results of #{directory} list #{listed}, not #{expected}" unless listed.sort == expected.sort

          failed = tests.find { |test| test["result"] == "failure" }
          raise "The results of #{directory} place its failure wrongly: #{failed}" unless placed?(failed)
        end
      end

      def placed?(failed)
        failed.nil? || (failed.values_at("file", "line") == FAILED_AT && failed["message"].include?("Expected"))
      end

      # A test file that does not parse is among the results' errors, with
      # the compiler's message at its line.
      # @behavior RT-037
      def verify_load_failure!(project)
        errors = results_of(project, "--dir", UNPARSED)["errors"]
        return if errors.any? { |error| error.values_at("file", "line") == UNPARSED_AT && error["message"] =~ /syntax/ }

        raise "The results of a test file that does not parse carry #{errors}"
      end

      # A run with no options takes the run file's: only its directory runs,
      # its results are written, and the run file is gone.
      # @behavior RT-038
      def verify_run_file!(project)
        run_file = File.join(user_dir(project), RUN_FILE)
        output, tests = Dir.mktmpdir { |dir| run_from(project, run_file, File.join(dir, "results.json")) }
        return if tests&.size == 2 && !File.exist?(run_file) && output.match?(/^2 runs,/)

        raise "A run with no options did not take the run file's and remove it:\n#{output}"
      ensure
        FileUtils.rm_f(run_file) if run_file
      end

      # Leaves the run file and runs with no options, answering the output and
      # the tests its results list.
      def run_from(project, run_file, results)
        FileUtils.mkdir_p(File.dirname(run_file))
        File.write(run_file, JSON.generate(["--dir", "#{RUNNER_TESTS}/skip", "--results", results]))
        output, = Runner.run(project)
        [output, File.exist?(results) ? JSON.parse(File.read(results))["tests"] : nil]
      end

      # What a run with these options wrote to --results.
      def results_of(project, *options)
        Dir.mktmpdir do |dir|
          path = File.join(dir, "results.json")
          output, = Runner.run(project, *options, "--results", path)
          raise "A run with #{options.join(" ")} wrote no results:\n#{output}" unless File.exist?(path)

          JSON.parse(File.read(path))
        end
      end

      # Where Godot keeps user:// for the project.
      def user_dir(project)
        output, = Open3.capture2e(Godot::EXECUTABLE, "--headless", "--path", project, "--script", USER_DIR_SCRIPT)
        output[USER_DIR, 1] or raise "Godot did not say where user:// is:\n#{output}"
      end
    end
  end
end
