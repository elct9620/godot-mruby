class AwaitTest < Minitest::Test
  # @behavior RS-094
  def test_a_method_godot_calls_returns_as_it_awaits
    sentinel, source = make_pair

    assert_nil sentinel.call(:watch, source)
    assert_equal [[:started], 1], [sentinel.log, source.get_signal_connection_list(:relayed).size]
  end

  # @behavior RS-095
  def test_an_awaiting_method_resumes_once_its_signal_is_emitted
    sentinel, source = make_pair
    sentinel.call(:watch, source)

    source.emit_signal(:relayed)

    assert_equal [:started, nil], sentinel.log
  end

  # @behavior RS-096
  def test_await_answers_what_the_signal_was_emitted_with
    answers = [[], [7], [7, 8]].map do |values|
      sentinel, source = make_pair
      sentinel.call(:watch, source)
      source.emit_signal(:relayed, *values)
      sentinel.log.last
    end

    assert_equal [nil, 7, [7, 8]], answers
  end

  # @behavior RS-097
  def test_a_block_godot_calls_back_may_await
    sentinel, source = make_pair
    sentinel.arm(source)

    source.emit_signal(:pinged)
    source.emit_signal(:relayed, 5)

    assert_equal [5], sentinel.log
  end

  # @behavior RS-098
  def test_await_outside_what_godot_calls_raises
    sentinel, source = make_pair

    assert_raises(FiberError) { sentinel.watch(source) }
  end

  private

  def make_pair
    source = autofree(Godot::Node.new)
    source.add_user_signal("relayed")
    source.add_user_signal("pinged")
    [autofree(Unit::Script::Sentinel.new), source]
  end
end
