class ReentryTest < Minitest::Test
  # @behavior RS-034
  def test_a_call_back_into_the_same_node_reaches_its_ruby_object
    echo = Unit::Script::Echo.new

    assert_equal :pinged, echo.call(:ping)
    assert_equal 7, echo.notified
  ensure
    echo&.free
  end

  # @behavior RS-035
  def test_a_call_arriving_while_a_node_initializes_reaches_the_object_initializing
    eager = Unit::Script::Eager.new

    assert_equal 3, eager.notified
  ensure
    eager&.free
  end

  # @behavior RS-036
  def test_a_call_arriving_before_a_file_defines_its_class_does_nothing
    Unit::Script::Early
    early = Unit::Script::EARLY_NODE

    early.notification(2)

    assert_equal 2, early.call(:notified)
  ensure
    early&.free
  end

  # @behavior RS-037
  def test_a_nodes_script_taken_away_while_its_ruby_runs_lets_the_call_finish
    detach = Unit::Script::Detach.new

    answer = detach.call(:detach)

    assert_equal :detached, answer
    assert_nil detach.get_script
  ensure
    detach&.free
  end
end
