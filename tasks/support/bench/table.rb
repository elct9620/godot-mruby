# frozen_string_literal: true

module Bench
  # Writes the bench's results as Markdown, a table for each part. Where main's
  # last results were measured with the same build in the same Godot, the
  # battle's and the calls' ratios are shown beside theirs: a ratio holds
  # across machines, as the milliseconds of two runners do not.
  module Table
    module_function

    # Every part of `results` as its table, after the library they were
    # measured with, and compared with `baseline` when it is given.
    def tabulate(results, baseline = nil)
      library = results["library"]
      ["Measured with a #{library["build"]} build in Godot #{library["godot"]}.",
       baseline ? "Ratios are compared with main's last bench." : "No bench from main with this build to compare.",
       tabulate_growth(results["growth"]),
       tabulate_battle(results["battle"], baseline&.fetch("battle", nil)),
       tabulate_spreads("call, ns", results["operations"], "%.0f", baseline&.fetch("operations", nil))].join("\n\n")
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

    def tabulate_battle(styles, baseline)
      [tabulate_spreads("battle, ms a round", styles, "%.1f", baseline), "",
       "The call-cost gate is to judge the recommended style's ratio."].join("\n")
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
