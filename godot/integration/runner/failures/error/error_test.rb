# A test that raises something other than an assertion from a method it calls:
# the runner has to report it as an error where it was raised, with the calls
# that led there, and fail the run.
class ErrorTest < Minitest::Test
  def test_raises_an_argument_error
    refuse_the_item
  end

  def refuse_the_item
    raise ArgumentError, "not an assertion"
  end
end
