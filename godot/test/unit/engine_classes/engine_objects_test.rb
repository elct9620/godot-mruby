class EngineObjectsTest < Minitest::Test
  # @behavior RG-008
  def test_an_engine_class_makes_the_engines_object
    node = autofree(Godot::Node.new)

    assert_instance_of Godot::Node, node
    assert_equal 0, node.get_child_count
  end

  # @behavior RG-009
  def test_an_engine_class_the_engine_cannot_make_raises_not_implemented_error
    assert_raises(NotImplementedError) { Godot::CanvasItem.new }
  end

  # @behavior RG-010
  def test_ruby_calls_an_engine_objects_method_by_its_name
    node = autofree(Godot::Node.new)

    node.set_process_priority(3)

    assert_equal 3, node.get_process_priority
  end

  # @behavior RG-011
  def test_a_name_the_engine_object_has_no_method_for_raises_no_method_error
    node = autofree(Godot::Node.new)

    assert_raises(NoMethodError) { node.no_such_engine_method }
  end

  # @behavior RG-012
  def test_a_freed_engine_object_raises_call_error
    node = Godot::Node.new
    node.free

    error = assert_raises(Godot::CallError) { node.get_child_count }
    assert_equal "Attempt to call function 'get_child_count' in base 'previously freed' on a null instance.",
                 error.message
  end

  # @behavior RG-013
  def test_a_reference_counted_object_lives_while_ruby_holds_it
    counted = Godot::RefCounted.new

    GC.start

    assert_operator counted.get_reference_count, :>=, 1
  end

  # @behavior RG-014
  def test_ruby_reads_and_writes_an_engine_property_by_its_name
    node = autofree(Godot::Node.new)

    node.process_priority = 5

    assert_equal 5, node.process_priority
  end

  # @behavior RG-015
  def test_an_engine_class_names_its_integer_constants
    assert_equal 13, Godot::Node::NOTIFICATION_READY
    assert_equal 4, Godot::Node::PROCESS_MODE_DISABLED
  end

  # @behavior RG-016
  def test_an_engine_singleton_answers_its_methods_on_its_class
    assert_equal Godot::Engine.get_physics_ticks_per_second, Godot::Engine.physics_ticks_per_second
    assert_operator Godot::Engine.get_physics_ticks_per_second, :>, 0
  end

  # @behavior RG-017
  def test_an_engine_class_answers_its_static_methods
    assert_in_delta 0.5, Godot::Tween.interpolate_value(0.0, 1.0, 0.5, 1.0, 0, 0)
  end

  # @behavior RG-018
  def test_a_call_the_engine_refuses_raises_call_error_as_gdscript_reports_it
    node = autofree(Godot::Node.new)

    too_few = assert_raises(Godot::CallError) { node.set_process_priority }
    wrong_type = assert_raises(Godot::CallError) { node.set_process_priority(nil) }

    assert_equal "Invalid call to function 'set_process_priority' in base 'Node'. Expected 1 argument(s).",
                 too_few.message
    assert_equal "Invalid type in function 'set_process_priority' in base 'Node'. " \
                 "Cannot convert argument 1 from Nil to int.", wrong_type.message
  end

  # @behavior RG-019
  def test_an_engine_method_named_like_a_ruby_method_is_reached_through_call
    peer = Godot::WebSocketPeer.new

    assert_equal Godot::WebSocketPeer, peer.send(:class)
    assert_kind_of Integer, peer.call(:send, [104, 105])
  end
end
