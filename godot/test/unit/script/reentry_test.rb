class ReentryTest < Minitest::Test
  # @behavior RS-034
  def test_a_call_back_into_the_same_node_reaches_its_ruby_object
    echo = autofree(Unit::Script::Echo.new)

    assert_equal :pinged, echo.call(:ping)
    assert_equal 7, echo.last_notification
  end

  # @behavior RS-035
  def test_a_call_arriving_while_a_node_initializes_reaches_the_object_initializing
    sprinter = autofree(Unit::Script::Sprinter.new)

    assert_equal 3, sprinter.last_notification
  end

  # @behavior RS-036
  def test_a_call_arriving_before_a_file_defines_its_class_does_nothing
    Unit::Script::Forerunner
    forerunner = autofree(Unit::Script::EARLY_NODE)

    forerunner.notification(2)

    assert_equal 2, forerunner.call(:last_notification)
  end

  # @behavior RS-037
  def test_a_nodes_script_taken_away_while_its_ruby_runs_lets_the_call_finish
    detacher = autofree(Unit::Script::Detacher.new)

    answer = detacher.call(:detach)

    assert_equal :detached, answer
    assert_nil detacher.get_script
  end
end
