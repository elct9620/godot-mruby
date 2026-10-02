# frozen_string_literal: true

module Godot
  module Runner
    # Tells GitHub Actions what a run's --results hold, when this runs there:
    # each failure and file that did not load as an error annotated at its
    # line in the repository, and the run's counts in the step summary. The
    # addon writes no such thing itself; this is the project's own CI.
    module Workflow
      # Where res:// sits in the repository.
      SOURCE = "godot"
      FAILED = %w[failure error].freeze

      module_function

      def report(results, env = ENV)
        return unless env["GITHUB_ACTIONS"] == "true"

        puts annotate_failures(results)
        File.write(env["GITHUB_STEP_SUMMARY"], summarize(results), mode: "a") if env["GITHUB_STEP_SUMMARY"]
      end

      # The annotations of the tests that failed and the files that did not load.
      def annotate_failures(results)
        failed = results.fetch("tests").select { |test| FAILED.include?(test["result"]) }
        failed.map { |test| annotate("#{test["class"]}##{test["name"]}", test) } +
          results.fetch("errors").map { |error| annotate("Did not load", error) }
      end

      # A workflow command annotating `entry`'s message at its file and line.
      def annotate(title, entry)
        place = { "file" => entry["file"]&.sub("res://", "#{SOURCE}/"), "line" => entry["line"], "title" => title }
        properties = place.compact.map { |key, value| "#{key}=#{escape_property(value)}" }.join(",")
        "::error #{properties}::#{escape_data(entry["message"].to_s)}"
      end

      def summarize(results)
        counts = results.fetch("tests").map { |test| test["result"] }.tally
        rows = %w[pass skip failure error].map { |result| "| #{result} | #{counts.fetch(result, 0)} |" }
        ["### Ruby tests", "", "| Result | Tests |", "|---|---|", *rows,
         "| did not load | #{results.fetch("errors").size} |", ""].join("\n")
      end

      # Escaped as GitHub's workflow commands read a message.
      def escape_data(text)
        text.gsub("%", "%25").gsub("\r", "%0D").gsub("\n", "%0A")
      end

      # Escaped as GitHub's workflow commands read a property's value.
      def escape_property(value)
        escape_data(value.to_s).gsub(":", "%3A").gsub(",", "%2C")
      end
    end
  end
end
