class EngineObjectsTest < Minitest::Test
  # @behavior RG-008
  def test_an_engine_class_makes_the_engines_object
    node = Godot::Node.new

    assert_instance_of Godot::Node, node
    assert_equal 0, node.get_child_count
  ensure
    node&.free
  end

  # @behavior RG-009
  def test_an_engine_class_the_engine_cannot_make_raises_not_implemented_error
    assert_raises(NotImplementedError) { Godot::CanvasItem.new }
  end

  # @behavior RG-010
  def test_ruby_calls_an_engine_objects_method_by_its_name
    node = Godot::Node.new

    node.set_process_priority(3)

    assert_equal 3, node.get_process_priority
  ensure
    node&.free
  end

  # @behavior RG-011
  def test_a_name_the_engine_object_has_no_method_for_raises_no_method_error
    node = Godot::Node.new

    assert_raises(NoMethodError) { node.no_such_engine_method }
  ensure
    node&.free
  end

  # @behavior RG-012
  def test_a_freed_engine_object_raises_call_error
    node = Godot::Node.new
    node.free

    assert_raises(Godot::CallError) { node.get_child_count }
  end

  # @behavior RG-013
  def test_a_reference_counted_object_lives_while_ruby_holds_it
    counted = Godot::RefCounted.new

    GC.start

    assert_operator counted.get_reference_count, :>=, 1
  end
end
