class SignalsTest < Minitest::Test
  # @behavior RD-001
  def test_a_declared_signal_is_one_the_node_has
    bell = Loader::Bell.new

    assert bell.has_signal(:rung)
  ensure
    bell&.free
  end

  # @behavior RD-002
  def test_a_declared_signal_carries_the_parameters_it_was_declared_with
    bell = Loader::Bell.new

    rung = bell.get_signal_list.find { |signal| signal["name"] == "rung" }

    assert_equal ["times"], rung["args"].map { |argument| argument["name"] }
  ensure
    bell&.free
  end

  # @behavior RD-003
  def test_a_declared_signal_reaches_what_connected_to_it
    bell = Loader::Bell.new
    bell.connect(:rung, method(:hear))

    bell.ring(3)

    assert_equal [3], @heard
  ensure
    bell&.free
  end

  def hear(times)
    (@heard ||= []) << times
  end
end
