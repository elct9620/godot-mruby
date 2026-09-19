class ValueTypesTest < Minitest::Test
  # @behavior RV-010
  def test_an_engine_value_type_crosses_as_a_value_of_its_class_under_godot
    node = Godot::Node2D.new
    node.position = Godot::Vector2.new(1, 2)

    assert_instance_of Godot::Vector2, node.position
    assert_equal Godot::Vector2.new(1, 2), node.position
  ensure
    node&.free
  end

  # @behavior RV-011
  def test_ruby_builds_a_value_with_the_engines_constructors
    assert_equal Godot::Vector2.new(3, 4), Godot::Vector2.new(Godot::Vector2i.new(3, 4))
    assert_equal Godot::Color.new(1, 0, 0), Godot::Color.new("red")
    assert_equal "Hero/Sword", Godot::NodePath.new("Hero/Sword").to_s
  end

  # @behavior RV-012
  def test_a_constructor_no_arguments_fit_raises_call_error_as_gdscript_reports_it
    error = assert_raises(Godot::CallError) { Godot::Vector2.new("x") }

    assert_equal "Invalid call. Nonexistent 'Vector2' constructor.", error.message
  end

  # @behavior RV-013
  def test_a_value_answers_its_members_methods_and_constants
    vector = Godot::Vector2.new(3, 4)

    assert_equal 3.0, vector.x
    assert_equal 5.0, vector.length
    assert_equal Godot::Vector2.new(0.6, 0.8), vector.normalized
    assert_equal Godot::Vector2.new(0, 0), Godot::Vector2::ZERO
    assert_equal Godot::Color.new(1, 0, 0), Godot::Color.from_rgba8(255, 0, 0)
  end

  # @behavior RV-014
  def test_a_value_answers_the_engines_operators
    vector = Godot::Vector2.new(1, 2)

    assert_equal Godot::Vector2.new(4, 6), vector + Godot::Vector2.new(3, 4)
    assert_equal Godot::Vector2.new(2, 4), vector * 2
    assert_equal Godot::Vector2.new(-1, -2), -vector
    refute_equal vector, Godot::Vector2.new(2, 1)
    error = assert_raises(TypeError) { vector + "x" }
    assert_equal "Invalid operands 'Vector2' and 'String' in operator '+'.", error.message
  end

  # @behavior RV-015
  def test_a_value_cannot_be_changed
    vector = Godot::Vector2.new(1, 2)

    assert_raises(FrozenError) { vector.y = 0 }
    assert_equal 2.0, vector.y
  end

  # @behavior RV-016
  def test_a_value_prints_as_the_engine_prints_it
    assert_equal "(1.0, 2.0)", Godot::Vector2.new(1, 2).to_s
  end
end
