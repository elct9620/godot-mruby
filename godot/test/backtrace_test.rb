class BacktraceTest < Minitest::Test
  FILE = "res://test/backtrace_test.rb".freeze

  # @behavior RT-020
  def test_an_error_names_only_the_calls_in_the_test_that_led_to_it
    error = assert_raises(ArgumentError) { refuse }
    frames = Minitest.filter_backtrace(error.backtrace)
    refute_empty frames
    assert frames.all? { |frame| frame[0, FILE.size] == FILE }, frames.inspect
  end

  private

  def refuse
    raise ArgumentError, "refused"
  end
end
