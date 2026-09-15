module Test
  module Loader
    module Support
      # A test class a test loads by name while the run is under way: it is
      # collected, and never runs. Running test/minitest/ makes Test::Minitest,
      # which Ruby inside Test finds before the framework.
      class LateTestCase < ::Minitest::Test
        def test_never_runs
          flunk "a test class loaded by name during the run ran"
        end
      end
    end
  end
end
