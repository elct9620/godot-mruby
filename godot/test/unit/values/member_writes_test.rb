class MemberWritesTest < Minitest::Test
  # @behavior RV-015
  def test_assigning_a_values_member_gives_its_variable_a_copy
    vector = Godot::Vector2.new(1, 2)
    kept = vector

    vector.y = 0

    assert_equal Godot::Vector2.new(1, 0), vector
    assert_equal Godot::Vector2.new(1, 2), kept
  end

  # @behavior RV-043
  def test_a_member_of_a_propertys_value_is_written_back
    mover = autofree(Unit::Values::Mover.new)

    mover.call(:lift)
    mover.call(:push)
    mover.position.x += 1

    assert_equal Godot::Vector2.new(2, 5), mover.position
  end

  # @behavior RV-044
  def test_a_member_of_a_member_is_written_back_through_each_value
    mover = autofree(Unit::Values::Mover.new)

    mover.call(:slide)

    assert_equal Godot::Vector2.new(7, 0), mover.position
  end

  # @behavior RV-045
  def test_a_member_of_a_containers_element_is_written_into_the_container
    engine_list = Godot::Array.new([Godot::Vector2.new(1, 1)])
    engine_table = Godot::Dictionary.new({ "spot" => Godot::Vector2.new(1, 1) })
    list = [Godot::Vector2.new(1, 1)]
    table = { spot: Godot::Vector2.new(1, 1) }

    engine_list[0].x = 3
    engine_table["spot"].x = 4
    list[0].x = 5
    table[:spot].x = 6

    assert_equal [3.0, 4.0, 5.0, 6.0], [engine_list[0].x, engine_table["spot"].x, list[0].x, table[:spot].x]
  end

  # @behavior RV-046
  def test_an_element_of_a_propertys_packed_array_is_written_back
    line = autofree(Godot::Line2D.new)
    line.points = Godot::PackedVector2Array.new([Godot::Vector2.new(0, 0)])

    line.points[0] = Godot::Vector2.new(3, 4)

    assert_equal Godot::Vector2.new(3, 4), line.points[0]
  end

  # @behavior RV-047
  def test_an_operator_assignment_to_a_member_writes_it_back_once
    mover = autofree(Unit::Values::Mover.new)
    mover.position = Godot::Vector2.new(1, 2)

    mover.call(:nudge_once)

    assert_equal Godot::Vector2.new(1, 6), mover.position
  end

  # @behavior RV-048
  def test_a_multiple_assignment_to_members_writes_each_back
    mover = autofree(Unit::Values::Mover.new)

    corner = mover.call(:place)

    assert_equal [Godot::Vector2.new(3, 0), Godot::Vector2.new(0, 4)], [mover.position, corner]
  end

  # @behavior RV-049
  def test_a_member_of_a_value_a_call_answers_cannot_be_assigned
    mover = autofree(Unit::Values::Mover.new)

    assert_raises(FrozenError) { mover.move_spot }
    assert_raises(FrozenError) { mover.spot(*[]).x = 2 }
  end

  # @behavior RV-015
  def test_a_value_a_variable_copied_leaves_the_property_unchanged
    mover = autofree(Unit::Values::Mover.new)

    moved = mover.call(:copy_moved)

    assert_equal [99.0, 0.0], [moved.x, mover.position.x]
  end

  # @behavior RV-050
  def test_with_answers_a_copy_with_members_changed
    vector = Godot::Vector2.new(1, 2)

    moved = vector.with(x: 5, y: 6)
    error = assert_raises(ArgumentError) { vector.with(z: 1) }

    assert_equal [Godot::Vector2.new(5, 6), Godot::Vector2.new(1, 2)], [moved, vector]
    assert_equal "Invalid assignment of property or key 'z' with value of type 'int' on a base object of type 'Vector2'.",
                 error.message
  end
end
