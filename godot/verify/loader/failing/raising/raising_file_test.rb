# A test using a constant whose file raises: the runner has to report the
# error at the line of that file, and fail the run.
class RaisingFileTest < Minitest::Test
  def test_uses_a_file_that_raises
    Loader::Raising
  end
end
