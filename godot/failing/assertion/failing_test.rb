# A test whose assertion fails: the runner has to report it where it failed,
# still run teardown, and fail the run.
class FailingTest < Minitest::Test
  def teardown
    puts "teardown ran after a failure"
  end

  def test_one_equals_two
    assert_equal 1, 2
  end
end
