class UtilitiesTest < Minitest::Test
  # @behavior RC-003
  def test_godots_game_math_is_a_module_function_of_godot
    assert_in_delta 5.0, Godot.lerp(0.0, 10.0, 0.5)
    assert_in_delta 3.0, Godot.move_toward(0, 10, 3)
    assert_in_delta 0.5, Godot.smoothstep(0, 1, 0.5)
    assert_in_delta 2.0, Godot.wrapf(12, 0, 10)
    assert_equal 10, Godot.clamp(15, 0, 10)
    assert_equal 5, Godot.snapped(7, 5)
    assert_in_delta Math::PI, Godot.deg_to_rad(180)
  end

  # @behavior RC-004
  def test_godots_random_numbers_come_from_the_engines_generator
    Godot.seed(42)

    assert_in_delta 0.11471347510815, Godot.randf, 1e-12
  end

  # @behavior RC-005
  def test_rubys_own_random_numbers_are_a_stream_of_their_own
    srand(1)
    alone = rand
    srand(1)

    Godot.seed(99)

    assert_equal alone, rand
  end

  # @behavior RC-007
  def test_godot_is_same_compares_as_the_engine_does
    node = autofree(Godot::Node.new)

    assert Godot.is_same(node, node)
    assert Godot.is_same(Godot::Vector2.new(1, 2), Godot::Vector2.new(1, 2))
  end
end
