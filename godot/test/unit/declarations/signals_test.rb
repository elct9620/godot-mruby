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

  # @behavior RD-007
  def test_a_signal_declared_again_with_other_parameters_is_refused
    assert_raises(ArgumentError) { Loader::Clashing }
  end

  # @behavior RD-008
  def test_a_signal_declared_again_as_it_stands_is_no_new_signal
    repeating = Loader::Repeating.new

    rung = repeating.get_signal_list.select { |signal| signal["name"] == "rung" }

    assert_equal 1, rung.size
  ensure
    repeating&.free
  end

  # @behavior RD-009
  def test_a_node_has_the_signal_the_class_its_class_extends_declared
    chime = Loader::Chime.new

    assert chime.has_signal(:rung)
  ensure
    chime&.free
  end

  def hear(times)
    (@heard ||= []) << times
  end
end
