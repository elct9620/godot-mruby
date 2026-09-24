class BattleTest < Minitest::Test
  def setup
    @battle = Godot::ResourceLoader.load("res://src/battle.tscn").instantiate
  end

  # @behavior RS-011
  def test_the_battle_sends_its_wave_once_it_is_ready
    add_child_autofree(@battle)

    assert_equal @battle.wave, @battle.enemies.size
  end

  # @behavior RS-030
  def test_the_enemies_march_on_the_base
    add_child_autofree(@battle)
    enemy = @battle.enemies.first
    start = enemy.position.x

    wait_physics_frames 2

    assert_operator enemy.position.x, :>, start
  end

  # @behavior RS-028
  def test_godot_gets_a_enemies_speed_as_a_float_and_its_health_as_an_integer
    add_child_autofree(@battle)
    enemy = @battle.enemies.first

    speed = enemy.call(:speed)
    health = enemy.call(:health)

    assert_equal [Float, 200.0], [speed.class, speed]
    assert_equal [Integer, 3], [health.class, health]
  end

  # @behavior RS-017
  def test_the_battle_ends_cleared_once_its_last_enemy_is_destroyed
    add_child_autofree(@battle)

    assert wait_for_signal(Godot::Signal.new(@battle, :finished), 3), "the battle did not end"

    assert_empty @battle.enemies
    refute @battle.breached
  end
end
