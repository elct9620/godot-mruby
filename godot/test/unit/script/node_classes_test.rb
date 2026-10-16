class NodeClassesTest < Minitest::Test
  # @behavior RS-031 RS-069
  def test_a_node_scripts_class_makes_a_node_carrying_its_script
    beacon = autofree(Unit::Script::Beacon.new(3))

    assert_instance_of Unit::Script::Beacon, beacon
    assert_equal 3, beacon.brightness
    beacon.set_rotation(1.5)
    assert_in_delta 1.5, beacon.get_rotation
    beacon.notification(9001)
    assert_equal 9001, beacon.last_notification
  end

  # @behavior RS-032 RS-070
  def test_a_node_that_fails_to_initialize_is_freed
    assert_raises(RuntimeError) { Unit::Script::Glitch.new }
    assert_raises(Godot::CallError) { Unit::Script::Glitch.last.get_child_count }
  end

  # @behavior RS-033
  def test_a_class_no_node_script_defines_makes_no_node
    assert_raises(NotImplementedError) { Class.new(Godot::Node).new }
  end

  # @behavior RS-081
  def test_a_class_inside_a_node_scripts_class_makes_a_node_carrying_a_script_of_its_own
    hound = autofree(Unit::Script::Kennel::Hound.new)

    assert_instance_of Unit::Script::Kennel::Hound, hound
    refute_nil hound.get_script
    hound.set_rotation(1.5)
    assert_in_delta 1.5, hound.get_rotation
  end

  # @behavior RS-082
  def test_godot_calls_an_inner_classs_method_on_its_node
    hound = autofree(Unit::Script::Kennel::Hound.new)

    assert_equal "stick", hound.call(:fetch)
  end

  # @behavior RS-083
  def test_an_inner_classs_node_has_the_properties_the_inner_class_exported
    names = autofree(Unit::Script::Kennel::Hound.new).get_property_list.map { |property| property["name"] }

    assert_includes names, "bark"
    refute_includes names, "capacity"
  end

  # @behavior RS-084
  def test_a_node_scripts_node_has_none_of_its_inner_classs_properties
    names = autofree(Unit::Script::Kennel.new).get_property_list.map { |property| property["name"] }

    assert_includes names, "capacity"
    refute_includes names, "bark"
  end

  # @behavior RS-085
  def test_an_inner_classs_node_has_the_signals_the_inner_class_declared
    hound = autofree(Unit::Script::Kennel::Hound.new)

    assert hound.has_signal(:howled)
  end
end
