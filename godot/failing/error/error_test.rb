# A test that raises something other than an assertion: the runner has to
# report it as an error where it was raised, and fail the run.
class ErrorTest < Minitest::Test
  def test_raises_an_argument_error
    raise ArgumentError, "not an assertion"
  end
end
