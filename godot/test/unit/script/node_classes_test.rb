class NodeClassesTest < Minitest::Test
  # @behavior RS-031
  def test_a_node_scripts_class_makes_a_node_carrying_its_script
    beacon = Unit::Script::Beacon.new(3)

    assert_instance_of Unit::Script::Beacon, beacon
    assert_equal 3, beacon.brightness
    beacon.set_rotation(1.5)
    assert_in_delta 1.5, beacon.get_rotation
    beacon.notification(9001)
    assert_equal 9001, beacon.notified
  ensure
    beacon&.free
  end

  # @behavior RS-032
  def test_a_node_that_fails_to_initialize_is_freed
    assert_raises(RuntimeError) { Unit::Script::Faulty.new }
    assert_raises(Godot::CallError) { Unit::Script::Faulty.last.get_child_count }
  end

  # @behavior RS-033
  def test_a_class_no_node_script_defines_makes_no_node
    assert_raises(NotImplementedError) { Class.new(Godot::Node).new }
  end
end
