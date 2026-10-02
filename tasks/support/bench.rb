# frozen_string_literal: true

require "json"
require "tmpdir"

require_relative "bench/battle"
require_relative "bench/operations"
require_relative "bench/project"
require_relative "bench/table"
require_relative "godot"

# Measures what a game's Ruby costs: as its project grows, in a generated
# project played and scanned headless at each size; per round of the game's
# battle against its GDScript twin; and per call of each kind a game makes.
# Reports the numbers and judges nothing. Backs tasks/bench.rake.
module Bench
  SIZES = [100, 1000].freeze
  SCENE = "res://bench.tscn"
  # What the bench scene prints: a measure's name and its microseconds.
  MEASURE = /^bench (\w+) (\d+)$/
  # What godot-rust prints as the library loads: the Godot running it, and
  # its safeguards, whose default is strict in a debug build and balanced in
  # a release one.
  LIBRARY = /^Initialize godot-rust \(API [^,]+, runtime ([^,]+), safeguards (\w+)\)$/
  BUILDS = { "strict" => "debug", "balanced" => "release" }.freeze

  module_function

  # Measures the sizes `BENCH_SIZES` lists in `env`, the battle and the
  # calls, prints their tables beside the results `BENCH_BASELINE` names, and
  # writes the measures as JSON where `BENCH_JSON` names a file.
  def report!(env)
    sizes = env.fetch("BENCH_SIZES", SIZES.join(",")).split(",").map { |size| Integer(size) }
    results = measure_all(sizes)
    puts Table.tabulate(results, read_baseline(env["BENCH_BASELINE"], results["library"]))
    File.write(env.fetch("BENCH_JSON"), JSON.pretty_generate(results)) if env.key?("BENCH_JSON")
  end

  # Every part's measures: the library's build, growth at `sizes`, the
  # battle and the calls.
  def measure_all(sizes)
    { "library" => describe_library, "growth" => measure(sizes), "battle" => Battle.measure,
      "operations" => Operations.measure }
  end

  # The results an earlier bench wrote at `path`, when they were measured with
  # `library`'s build in its Godot; a ratio from another build compares
  # nothing.
  def read_baseline(path, library)
    return unless path && File.exist?(path)

    baseline = JSON.parse(File.read(path))
    baseline if baseline["library"] == library
  end

  # The measures of each size, keyed by its number of library files, in
  # microseconds.
  def measure(sizes = SIZES)
    sizes.to_h do |files|
      Dir.mktmpdir do |project|
        Project.generate(project, files)
        [files, measure_project(project)]
      end
    end
  end

  # Opens the editor on the project and plays the bench scene, each timed as
  # a whole, beside what the scene timed inside Ruby.
  def measure_project(project)
    editor_scan = time { Godot.run_editor(project) }
    measures = nil
    play = time { measures = play(project) }
    measures.merge("play" => play, "editor_scan" => editor_scan)
  end

  # Plays the bench scene of a generated project, answering what it timed
  # inside Ruby.
  def play(project)
    output, = Godot.run_scene(project, SCENE, "--quit-after", "10")
    errors = output.lines.grep(Godot::FAILED)
    raise "The bench scene failed:\n#{output}" unless errors.empty?

    measures = output.scan(MEASURE).to_h { |name, usec| [name, Integer(usec)] }
    raise "The bench scene measured nothing:\n#{output}" if measures.empty?

    measures
  end

  # The microseconds the block took.
  def time
    started = Process.clock_gettime(Process::CLOCK_MONOTONIC, :microsecond)
    yield
    Process.clock_gettime(Process::CLOCK_MONOTONIC, :microsecond) - started
  end

  # The build of the library the addon of `source` holds, and the Godot it
  # runs in, so numbers from a debug build are not read as a release's. They
  # are read as a battle project opens, which loads the library and nothing
  # that fails.
  def describe_library(source = Godot::PROJECT)
    output = Dir.mktmpdir do |project|
      Battle.generate(project, Battle::STYLES.first, source)
      Godot.run_editor(project).first
    end
    runtime, safeguards = output.match(LIBRARY)&.captures
    raise "godot-rust did not name its build:\n#{output}" unless safeguards

    { "build" => BUILDS.fetch(safeguards, safeguards), "godot" => runtime }
  end
end
