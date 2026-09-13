module Test
  module Support
    # A test class a test loads by name while the run is under way: it is
    # collected, and never runs.
    class LateTestCase < Minitest::Test
      def test_never_runs
        flunk "a test class loaded by name during the run ran"
      end
    end
  end
end
