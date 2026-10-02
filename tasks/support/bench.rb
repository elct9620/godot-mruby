# frozen_string_literal: true

require "json"
require "tmpdir"

require_relative "bench/project"
require_relative "godot"

# Measures what a game's Ruby costs as its project grows: a generated project,
# played and scanned headless, at each size. Reports the numbers and judges
# nothing. Backs tasks/bench.rake.
module Bench
  SIZES = [100, 1000].freeze
  SCENE = "res://bench.tscn"
  # What the bench scene prints: a measure's name and its microseconds.
  MEASURED = /^bench (\w+) (\d+)$/

  module_function

  # Measures the sizes `BENCH_SIZES` lists in `env`, prints the table, and
  # writes the measures as JSON where `BENCH_JSON` names a file.
  def report!(env)
    sizes = env.fetch("BENCH_SIZES", SIZES.join(",")).split(",").map { |size| Integer(size) }
    results = measure(sizes)
    puts tabulate(results)
    File.write(env.fetch("BENCH_JSON"), JSON.pretty_generate(results)) if env.key?("BENCH_JSON")
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

    measures = output.scan(MEASURED).to_h { |name, usec| [name, Integer(usec)] }
    raise "The bench scene measured nothing:\n#{output}" if measures.empty?

    measures
  end

  # The microseconds the block took.
  def time
    started = Process.clock_gettime(Process::CLOCK_MONOTONIC, :microsecond)
    yield
    Process.clock_gettime(Process::CLOCK_MONOTONIC, :microsecond) - started
  end

  # The measures as a Markdown table in milliseconds, a column for each size.
  def tabulate(results)
    sizes = results.keys
    header = ["| ms |", *sizes.map { |size| " #{size} files |" }].join
    rows = results.values.first.keys.map { |name| format_row(name, results.values.map { |measures| measures[name] }) }
    [header, "|---|#{"---|" * sizes.size}", *rows].join("\n")
  end

  def format_row(name, usecs)
    ["| #{name} |", *usecs.map { |usec| format(" %.1f |", usec / 1000.0) }].join
  end
end
