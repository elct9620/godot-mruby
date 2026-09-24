# frozen_string_literal: true

module Godot
  # Runs the integration-test project's Ruby tests through the addon's runner
  # scene and reads what the runs report. Part of Godot.verify!.
  module Runner
    RUNNER_SCENE = "res://addons/godot_mruby/runner.tscn"
    # A run lasts as many frames as its tests wait; this only stops a runner
    # that never quits from hanging the check.
    RUNNER_FRAMES = "600"
    TESTS = "res://test"
    PASSED = /^(\d+) runs, \d+ assertions, 0 failures, 0 errors, \d+ skips$/
    # What Godot warns at exit, in 4.6's wording and 4.7's alike, of an object
    # nothing freed, such as a node a test made and never freed.
    LEAKED = "leaked at exit"
    RUNNER_TESTS = "res://integration/runner"
    # Runs that have to pass, each with the runner's options and the summary it
    # has to end on: a directory whose one test passes and the other skips,
    # and runs narrowed by a filter.
    PASSING = {
      %W[--dir #{RUNNER_TESTS}/skip] => /^2 runs, \d+ assertions, 0 failures, 0 errors, 1 skips$/,
      %W[--dir #{RUNNER_TESTS}/order --include AlphaOrderTest#test_a] =>
        /^1 runs, \d+ assertions, 0 failures, 0 errors, 0 skips$/,
      %W[--dir #{RUNNER_TESTS}/skip --exclude SkipTest#test_skips] =>
        /^1 runs, \d+ assertions, 0 failures, 0 errors, 0 skips$/
    }.freeze
    # A test directory whose tests print their names as they run, the seeds
    # tried on it, and how a run prints its seed and each test it ran.
    ORDER = "#{RUNNER_TESTS}/order".freeze
    SEEDS = %w[1 2 3 4 5].freeze
    SEED = /^Run options: --seed (\d+)$/
    RAN = /ran (\w+#test_\w+)/
    # A test directory whose one test leaves an orphan node, and what the run
    # has to warn of it.
    ORPHAN = "#{RUNNER_TESTS}/orphan".freeze
    ORPHAN_WARNING = "WARNING: OrphanTest#test_leaves_an_orphan leaves 1 orphan node"

    module_function

    def verify!(project)
      verify_pass!(project)
      verify_passing!(project)
      verify_seed!(project)
      verify_shuffled!(project)
      verify_orphan!(project)
    end

    # Runs the project's Ruby tests and requires a pass that ran at least one,
    # since a run that finds nothing passes too, and that leaked nothing.
    # @behavior RT-001
    def verify_pass!(project)
      output, status = run(project, "--dir", TESTS)
      return if status.success? && output[PASSED, 1].to_i.positive? && !output.include?(LEAKED)

      raise "The Ruby tests under #{TESTS} did not pass, or leaked what they made:\n#{output}"
    end

    # Makes each run that has to pass and requires the summary it has to end
    # on.
    # @behavior RT-009 RT-024 RT-025
    def verify_passing!(project)
      PASSING.each do |options, summary|
        output, status = run(project, *options)
        next if status.success? && output.match?(summary)

        raise "The Ruby tests with #{options.join(" ")} did not end as they should:\n#{output}"
      end
    end

    # Runs the order directory without a seed, which has to print the one it
    # used, then again with that seed, which has to repeat the order.
    # @behavior RT-031 RT-032
    def verify_seed!(project)
      output, = run(project, "--dir", ORDER)
      seed = output[SEED, 1]
      raise "A run of #{ORDER} did not print its seed:\n#{output}" unless seed

      again, = run(project, "--dir", ORDER, "--seed", seed)
      return if order_of(output).any? && order_of(again) == order_of(output)

      raise "The seed #{seed} did not repeat the order of #{ORDER}:\n#{output}\n#{again}"
    end

    # Runs the order directory with each seed; one of them has to run its
    # tests out of name order.
    # @behavior RT-033
    def verify_shuffled!(project)
      orders = SEEDS.map { |seed| order_of(run(project, "--dir", ORDER, "--seed", seed).first) }
      return if orders.any? { |order| order.any? && order != order.sort }

      raise "No seed of #{SEEDS.join(", ")} ran #{ORDER} out of name order: #{orders}"
    end

    # Runs the orphan directory, whose test has to be warned of and still
    # pass.
    # @behavior RT-035
    def verify_orphan!(project)
      output, status = run(project, "--dir", ORPHAN)
      return if status.success? && output.include?(ORPHAN_WARNING)

      raise "A run of #{ORPHAN} did not warn of its orphan and pass:\n#{output}"
    end

    def order_of(output)
      output.scan(RAN).flatten
    end

    # The expected lines that do not appear in the output after the one before
    # them.
    def missing_in_order(output, expected)
      position = 0
      expected.reject do |line|
        found = output.index(line, position)
        position = found + line.size if found
        found
      end
    end

    def run(project, *options)
      Godot.run_scene(project, RUNNER_SCENE, "--quit-after", RUNNER_FRAMES, "--", *options)
    end
  end
end
