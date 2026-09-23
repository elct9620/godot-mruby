# Loads a test class by name while the run is under way: the run has to pass
# with this test alone, since the loaded class's test flunks if it runs.
class LateTest < Minitest::Test
  def test_loads_a_test_class_by_name
    assert_includes Minitest::Runnable.runnables, LateTestCase
  end
end
