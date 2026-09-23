class EngineClassesTest < Minitest::Test
  # @behavior RG-001
  def test_an_engine_class_is_a_class_under_godot
    assert_instance_of Class, Godot::Sprite2D
    assert_equal "Godot::Sprite2D", Godot::Sprite2D.to_s
  end

  # @behavior RG-002
  def test_an_engine_class_inherits_from_its_engine_parent
    assert_equal Godot::Node2D, Godot::Sprite2D.superclass
  end

  # @behavior RG-002
  def test_the_engine_root_class_is_godots_own
    assert_equal "Godot::Object", Godot::Node.superclass.to_s
  end

  # @behavior RG-003
  def test_a_name_the_engine_has_no_class_for_raises_name_error
    assert_raises(NameError) { Godot::NoSuchEngineClass }
  end

  # @behavior RG-004
  def test_an_engine_class_stays_after_a_file_that_first_used_it_raises
    assert_raises(RuntimeError) { Unit::EngineClasses::EngineRaising }
    assert Godot.const_defined?(:Marker3D)
  end
end
