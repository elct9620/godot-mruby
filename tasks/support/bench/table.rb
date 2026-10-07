# frozen_string_literal: true

module Bench
  # Writes the bench's results as Markdown, a table for each part. Where main's
  # last results were measured with the same build in the same Godot, the
  # battle's and the calls' ratios are shown beside theirs: a ratio holds
  # across machines, as the milliseconds of two runners do not.
  module Table
    module_function

    # Every part of `results` as its table, after the library they were
    # measured with, and compared with `baseline` when it is given. Growth
    # measured at no size has no table.
    def tabulate(results, baseline = nil)
      growth = results["growth"]
      [describe_measured(results), describe_baseline(baseline, results["cpu"]),
       (tabulate_growth(growth) unless growth.empty?),
       tabulate_battle(results, baseline),
       tabulate_spreads("call, ns", results["operations"], "%.0f", baseline&.fetch("operations", nil))]
        .compact.join("\n\n")
    end

    # The build, the Godot and the CPU the results were measured with.
    def describe_measured(results)
      library = results["library"]
      "Measured with a #{library["build"]} build in Godot #{library["godot"]} on #{results["cpu"]}."
    end

    # Which bench of main's the ratios are compared with, and its CPU where it
    # ran on another, since the ratios move with the CPU too.
    def describe_baseline(baseline, cpu)
      return "No bench from main with this build to compare." unless baseline
      return "Ratios are compared with main's last bench." if baseline["cpu"] == cpu

      ran_on = baseline.key?("cpu") ? "ran on #{baseline["cpu"]}" : "named no CPU"
      "Ratios are compared with main's last bench, which #{ran_on}, so they may differ by the CPU too."
    end

    # The growth measures in milliseconds, a column for each size.
    def tabulate_growth(sizes)
      header = ["| ms |", *sizes.keys.map { |size| " #{size} files |" }].join
      rows = sizes.values.first.keys.map { |name| format_growth(name, sizes.values) }
      [header, "|---|#{"---|" * sizes.size}", *rows].join("\n")
    end

    def format_growth(name, measures)
      ["| #{name} |", *measures.map { |measure| format(" %.1f |", measure[name] / 1000.0) }].join
    end

    # The battle's table beside main's, and the call-cost gate's verdict
    # against main's bench from the same CPU: a battle ratio past main's by
    # more than REGRESSION is marked, since the battle's ratio holds within a
    # few percent on one CPU where a single call's does not.
    def tabulate_battle(results, baseline)
      styles = results["battle"]
      [tabulate_spreads("battle, ms a round", styles, "%.1f", baseline&.fetch("battle", nil)), "",
       judge_battle(styles, comparable(baseline, results["cpu"])&.fetch("battle", nil))].join("\n")
    end

    # How far past main's ratio a battle's may grow before the gate marks it.
    REGRESSION = 0.10

    def judge_battle(styles, baseline)
      return "The call-cost gate judges only beside main's bench from the same CPU." unless baseline

      regressed = styles.select do |name, measures|
        main = baseline.dig(name, "ratio", "median")
        main && measures["ratio"]["median"] > main * (1 + REGRESSION)
      end
      return "The call-cost gate passes: no battle's ratio grew past main's by more than 10%." if regressed.empty?

      "**The call-cost gate warns: #{regressed.keys.join(" and ")} grew past main's by more than 10%.** " \
        "One bench can be off by itself, so run it again before trusting this."
    end

    # `baseline` when it was measured on `cpu`, the only one whose ratios the
    # gate compares.
    def comparable(baseline, cpu)
      baseline if baseline && baseline["cpu"] == cpu
    end

    # Rows of each language's measure and the ratio, each the median with the
    # range of the pairs, and main's ratio where `baseline` holds the row.
    def tabulate_spreads(heading, rows, unit, baseline)
      header = "| #{heading} | Ruby | GDScript | Ruby ÷ GDScript |#{" main (change) |" if baseline}"
      lines = rows.map do |name, measures|
        cells = format_measures(measures, unit)
        cells << compare_ratio(measures["ratio"], baseline.dig(name, "ratio")) if baseline
        "| #{name} | #{cells.join(" | ")} |"
      end
      [header, "|---|---|---|---|#{"---|" if baseline}", *lines].join("\n")
    end

    def format_measures(measures, unit)
      measures.map { |measure, spread| format_spread(spread, measure == "ratio" ? "%.1f×" : unit) }
    end

    def format_spread(spread, unit)
      "#{format(unit, spread["median"])} (#{format(unit, spread["min"])}–#{format(unit, spread["max"])})"
    end

    # main's median ratio, and how far this one's lies from it.
    def compare_ratio(ratio, main)
      return "–" unless main

      change = (ratio["median"] - main["median"]) / main["median"] * 100
      format("%<main>.1f× (%<change>+.0f%%)", main: main["median"], change: change)
    end
  end
end
