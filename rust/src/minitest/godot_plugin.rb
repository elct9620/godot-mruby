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

  # Writes each test's result, and each test file that did not load, as JSON
  # to the file `--results` names, for the editor's test panel to list. The
  # file and its directory are written through the engine, so `user://`
  # names one too.
  class ResultReporter < AbstractReporter
    RESULTS = { "." => "pass", "S" => "skip", "F" => "failure", "E" => "error" }.freeze

    def initialize(options)
      @path = options[:results]
      @errors = options[:load_failures] || []
      @tests = []
    end

    def record(result)
      failure = result.failure
      file, line = failure ? failure.file_and_line : [nil, nil]
      @tests << {
        "class" => result.class.to_s,
        "name" => result.name,
        "result" => RESULTS[result.result_code],
        "message" => failure && failure.message,
        "file" => file,
        "line" => line
      }
    end

    def report
      Godot::DirAccess.make_dir_recursive_absolute(@path[0, @path.rindex("/")])
      file = Godot::FileAccess.open(@path, Godot::FileAccess::WRITE)
      return Godot.push_error("The test results were not written to #{@path}") unless file

      file.store_string(Godot::JSON.stringify({ "tests" => @tests, "errors" => @errors }, "  "))
      file.close
    end
  end

  def self.plugin_godot_init(options)
    reporter << LogReporter.new(options)
    reporter << ResultReporter.new(options) if options[:results]
  end
end

Minitest.extensions << "godot"
