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

  # @behavior RG-018
  def test_a_call_of_a_method_only_the_objects_own_engine_class_has_is_refused_as_gdscript_reports_it
    sprite = autofree(Godot::Sprite2D.new)
    sprite.set_script(Godot::ResourceLoader.load("res://test/unit/engine_classes/scout.rb"))

    assert_equal "Invalid call to function 'get_rect' in base 'Sprite2D'. Expected 0 argument(s).",
                 sprite.call(:refusal)
  end

  # @behavior RG-019
  def test_an_engine_method_named_like_a_ruby_method_is_reached_through_call
    peer = Godot::WebSocketPeer.new

    assert_equal Godot::WebSocketPeer, peer.send(:class)
    assert_kind_of Integer, peer.call(:send, [104, 105])
  end

  # @behavior RG-021
  def test_a_method_only_some_of_a_node_classs_engine_objects_have_stays_theirs
    sprite = autofree(Godot::Sprite2D.new)
    plain = autofree(Godot::Node2D.new)
    [sprite, plain].each { |node| node.set_script(Godot::ResourceLoader.load("res://test/unit/engine_classes/scout.rb")) }

    reached = [sprite, plain].map { |node| node.call(:reaches_rect) }

    assert_equal [true, false], reached
  end

  # @behavior RG-022
  def test_a_node_classs_object_stands_only_for_an_engine_object_of_the_class_it_extends
    node = autofree(Godot::Node.new)
    freed = Godot::Node2D.new
    freed.free

    assert_raises(TypeError) { Unit::EngineClasses::Scout.send(:__allocate__, node) }
    assert_raises(TypeError) { Unit::EngineClasses::Scout.send(:__allocate__, freed) }
  end

  # @behavior RG-023
  def test_an_engine_object_inspects_as_godot_prints_it
    node = autofree(Godot::Node.new)
    held = autofree(Godot::Node.new)
    held.instance_variable_set(:@hp, 3)
    looped = autofree(Godot::Node.new)
    looped.instance_variable_set(:@me, looped)
    freed = Godot::Node.new
    freed.free

    inspected = [node, held, looped, freed].map(&:inspect)

    assert_equal ["#<Godot::Node <Node##{node.get_instance_id}>>",
                  "#<Godot::Node <Node##{held.get_instance_id}> @hp=3>",
                  "#<Godot::Node <Node##{looped.get_instance_id}> @me=#<Godot::Node <Node##{looped.get_instance_id}> ...>>",
                  "#<Godot::Node <Freed Object>>"], inspected
  end

  # @behavior RG-024
  def test_an_engine_objects_copy_stands_for_the_same_engine_object
    node = autofree(Godot::Node.new)
    node.instance_variable_set(:@hp, 3)

    copies = [node.dup, node.clone]

    copies.each do |copy|
      refute_same node, copy
      assert_instance_of Godot::Node, copy
      assert_equal node, copy
      assert_equal 3, copy.instance_variable_get(:@hp)
    end
  end

  # @behavior RG-025
  def test_a_node_classs_object_refuses_to_be_copied
    scout = autofree(Unit::EngineClasses::Scout.new)

    assert_raises(TypeError) { scout.dup }
    assert_raises(TypeError) { scout.clone }
  end

  # @behavior RG-026
  def test_an_engine_class_allocates_no_empty_object
    assert_raises(TypeError) { Godot::Node.allocate }
    assert_raises(TypeError) { Unit::EngineClasses::Scout.allocate }
  end
end
