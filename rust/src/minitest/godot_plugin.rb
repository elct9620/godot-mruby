# The extension's minitest plugin: every run writes what went wrong to
# Godot's log, where a reader of the run looks for it, besides printing it.
module Minitest
  # Writes each test that did not pass to Godot's log at the Ruby line it went
  # wrong at, after the summary. A filter that leaves no test to run is
  # written there too and fails the run. `error` is the extension's.
  class LogReporter < AbstractReporter
    def initialize(options)
      @filter = options[:include]
      @count = 0
      @problems = []
    end

    def record(result)
      @count += 1
      @problems << result unless result.passed? || result.skipped?
    end

    def report
      error("Nothing ran for filter: #{@filter}", nil, nil) if nothing_ran?
      @problems.each do |result|
        error("#{result.class}##{result.name}: #{result.failure.message}", *result.failure.file_and_line)
      end
    end

    def passed?
      !nothing_ran?
    end

    private

    def nothing_ran?
      @filter && @count == 0
    end
  end

  def self.plugin_godot_init(options)
    reporter << LogReporter.new(options)
  end
end

Minitest.extensions << "godot"
