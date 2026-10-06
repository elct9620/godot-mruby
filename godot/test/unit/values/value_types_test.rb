class ValueTypesTest < Minitest::Test
  # @behavior RV-010
  def test_an_engine_value_type_crosses_as_a_value_of_its_class_under_godot
    node = autofree(Godot::Node2D.new)
    node.position = Godot::Vector2.new(1, 2)

    assert_instance_of Godot::Vector2, node.position
    assert_equal Godot::Vector2.new(1, 2), node.position
  end

  # @behavior RV-010
  def test_another_value_type_crosses_as_a_value_of_its_class
    node = autofree(Godot::Node2D.new)
    node.modulate = Godot::Color.new(1, 0, 0)

    assert_instance_of Godot::Color, node.modulate
    assert_equal Godot::Color.new(1, 0, 0), node.modulate
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
    other = assert_raises(Godot::CallError) { Godot::Vector2i.new("x") }
    assert_equal "Invalid call. Nonexistent 'Vector2i' constructor.", other.message
  end

  # @behavior RV-013
  def test_a_value_answers_its_members_methods_and_constants
    vector = Godot::Vector2.new(3, 4)

    assert_equal 3.0, vector.x
    assert_equal vector, Godot::Rect2.new(1, 2, 3, 4).size
    assert_equal vector, Godot::Transform2D.new(0.0, vector).origin
    assert_equal 5.0, vector.length
    assert_equal Godot::Vector2.new(0.6, 0.8), vector.normalized
    assert_equal Godot::Vector2.new(0, 0), Godot::Vector2::ZERO
    assert_equal Godot::Color.new(1, 0, 0), Godot::Color.from_rgba8(255, 0, 0)
    assert_equal 3, Godot::Vector2i.new(3, 4).x
    assert_equal 7.0, Godot::Vector3.new(2, 3, 6).length
  end

  # @behavior RV-014
  def test_a_value_answers_the_engines_operators
    vector = Godot::Vector2.new(1, 2)

    assert_equal Godot::Vector2.new(4, 6), vector + Godot::Vector2.new(3, 4)
    assert_equal Godot::Vector2.new(2, 4), vector * 2
    assert_equal Godot::Vector2.new(-1, -2), -vector
    assert_equal vector, +vector
    assert_equal Godot::Vector2.new(0.5, 1), vector * 0.5
    assert_operator vector, :<, Godot::Vector2.new(1, 3)
    assert_operator vector, :>=, Godot::Vector2.new(0, 9)
    refute_equal vector, Godot::Vector2.new(2, 1)
    error = assert_raises(TypeError) { vector + "x" }
    assert_equal "Invalid operands 'Vector2' and 'String' in operator '+'.", error.message
  end

  # @behavior RV-014
  def test_another_value_type_answers_the_engines_operators
    vector = Godot::Vector3.new(1, 2, 3)

    assert_equal Godot::Vector3.new(2, 4, 6), vector + vector
    assert_equal Godot::Vector3.new(2, 4, 6), vector * 2
    assert_equal Godot::Vector3.new(-1, -2, -3), -vector
    refute_equal vector, Godot::Vector3.new(3, 2, 1)
    error = assert_raises(TypeError) { vector + "x" }
    assert_equal "Invalid operands 'Vector3' and 'String' in operator '+'.", error.message
  end

  # @behavior RV-013
  def test_a_value_answers_a_member_the_same_each_time_it_is_read
    vector = Godot::Vector2.new(3, 4)

    assert_equal [4.0, 4.0], [vector.y, vector.y]
  end

  # @behavior RV-014
  def test_a_value_answers_an_operator_the_same_each_time_it_is_applied
    vector = Godot::Vector2.new(3, 4)
    other = Godot::Vector2.new(1, 2)

    assert_equal [Godot::Vector2.new(2, 2), Godot::Vector2.new(2, 2)], [vector - other, vector - other]
  end

  # @behavior RV-024
  def test_a_division_or_modulo_by_zero_raises_zero_division_error_as_gdscript_reports_it
    vector = Godot::Vector2i.new(4, 6)
    zero = Godot::Vector2i.new(0, 1)
    assert_equal Godot::Vector2i.new(2, 3), vector / Godot::Vector2i.new(2, 2)
    assert_equal Godot::Vector2i.new(0, 0), vector % Godot::Vector2i.new(2, 2)

    divided = assert_raises(ZeroDivisionError) { vector / zero }
    remainder = assert_raises(ZeroDivisionError) { vector % zero }

    assert_equal "Division by zero error in operator '/'.", divided.message
    assert_equal "Modulo by zero error in operator '%'.", remainder.message
  end

  # @behavior RV-015
  def test_a_value_cannot_be_changed
    vector = Godot::Vector2.new(1, 2)

    assert_raises(FrozenError) { vector.y = 0 }
    assert_equal 2.0, vector.y
    other = Godot::Vector3.new(1, 2, 3)
    assert_raises(FrozenError) { other.z = 0 }
    assert_equal 3.0, other.z
  end

  # @behavior RV-016
  def test_a_value_prints_as_the_engine_prints_it
    assert_equal "(1.0, 2.0)", Godot::Vector2.new(1, 2).to_s
    assert_equal "(1, 2)", Godot::Vector2i.new(1, 2).to_s
  end

  # @behavior RV-023
  def test_a_value_responds_to_its_members_and_methods
    vector = Godot::Vector3.new(1, 2, 3)

    assert_respond_to vector, :z
    assert_respond_to vector, :cross
    refute_respond_to vector, :fly
    assert_respond_to Godot::Vector2.new(1, 2), :x
    assert_respond_to Godot::Vector2.new(1, 2), :length
  end

  # @behavior RV-025
  def test_equal_values_are_one_hash_key
    table = { Godot::Vector2.new(1, 2) => :vector, Godot::Color.new(1, 0, 0) => :color }

    assert_equal :vector, table[Godot::Vector2.new(1, 2)]
    assert_equal :color, table[Godot::Color.new(1, 0, 0)]
    assert_nil table[Godot::Vector2i.new(1, 2)]
  end

  # @behavior RV-022
  def test_a_values_method_the_engine_refuses_raises_call_error_as_gdscript_reports_it
    error = assert_raises(Godot::CallError) { Godot::Vector2.new(1, 2).distance_to("x") }

    assert_equal "Invalid type in function 'distance_to' in base 'Vector2'. " \
                 "Cannot convert argument 1 from String to Vector2.", error.message
  end

  # @behavior RV-026
  def test_a_values_copy_is_another_value_equal_to_it
    vector = Godot::Vector2.new(1, 2)
    color = Godot::Color.new(1, 0, 0)
    color.instance_variable_set(:@tag, :red)

    copies = [vector.dup, color.clone, Godot::Transform3D.new.dup]

    refute_same vector, copies[0]
    assert_equal vector, copies[0]
    refute_same color, copies[1]
    assert_equal color, copies[1]
    assert_equal :red, copies[1].instance_variable_get(:@tag)
    assert_equal Godot::Transform3D.new, copies[2]
  end

  # @behavior RV-027
  def test_a_value_types_class_allocates_no_empty_value
    assert_raises(TypeError) { Godot::Vector2.allocate }
    assert_raises(TypeError) { Godot::Color.allocate }
  end
end
