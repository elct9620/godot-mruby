# frozen_string_literal: true

require "json"
require "tmpdir"

require_relative "bench/battle"
require_relative "bench/operations"
require_relative "bench/project"
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
  # calls, prints their tables, and writes the measures as JSON where
  # `BENCH_JSON` names a file.
  def report!(env)
    sizes = env.fetch("BENCH_SIZES", SIZES.join(",")).split(",").map { |size| Integer(size) }
    results = { "library" => describe_library, "growth" => measure(sizes), "battle" => Battle.measure,
                "operations" => Operations.measure }
    puts tabulate_results(results)
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

  # The measures as a Markdown table in milliseconds, a column for each size.
  def tabulate(results)
    sizes = results.keys
    header = ["| ms |", *sizes.map { |size| " #{size} files |" }].join
    rows = results.values.first.keys.map { |name| format_row(name, results.values.map { |measures| measures[name] }) }
    [header, "|---|#{"---|" * sizes.size}", *rows].join("\n")
  end

  # The build of the library the addon of `source` holds, and the Godot it
  # runs in, so numbers from a debug build are not read as a release's.
  def describe_library(source = Godot::PROJECT)
    output, = Godot.run_project(source)
    runtime, safeguards = output.match(LIBRARY)&.captures
    raise "godot-rust did not name its build:\n#{output}" unless safeguards

    { "build" => BUILDS.fetch(safeguards, safeguards), "godot" => runtime }
  end

  # Each part of the results as its Markdown table, after the library they
  # were measured with.
  def tabulate_results(results)
    library = results["library"]
    ["Measured with a #{library["build"]} build in Godot #{library["godot"]}.",
     tabulate(results["growth"]), tabulate_battle(results["battle"]),
     tabulate_operations(results["operations"])].join("\n\n")
  end

  # The battle's styles as a Markdown table: a round's milliseconds in each
  # language and the ratio.
  def tabulate_battle(styles)
    [tabulate_spreads("battle, ms a round", styles, "%.1f"), "",
     "The call-cost gate is to judge the recommended style's ratio."].join("\n")
  end

  # The calls as a Markdown table: one call's nanoseconds in each language
  # and the ratio.
  def tabulate_operations(calls)
    tabulate_spreads("call, ns", calls, "%.0f")
  end

  # Rows of each language's measure and the ratio, each the median with the
  # range of the pairs.
  def tabulate_spreads(heading, rows, unit)
    lines = rows.map do |name, measures|
      cells = measures.map { |measure, spread| format_spread(spread, measure == "ratio" ? "%.1f×" : unit) }
      "| #{name} | #{cells.join(" | ")} |"
    end
    ["| #{heading} | Ruby | GDScript | Ruby ÷ GDScript |", "|---|---|---|---|", *lines].join("\n")
  end

  def format_spread(spread, unit)
    "#{format(unit, spread["median"])} (#{format(unit, spread["min"])}–#{format(unit, spread["max"])})"
  end

  def format_row(name, usecs)
    ["| #{name} |", *usecs.map { |usec| format(" %.1f |", usec / 1000.0) }].join
  end
end
