class BattleTest < Minitest::Test
  def setup
    @battle = Godot::ResourceLoader.load("res://src/battle.tscn").instantiate
  end

  # @behavior RS-011
  def test_the_battle_sends_its_wave_once_it_is_ready
    add_child_autofree(@battle)

    assert_equal @battle.wave, @battle.raiders.size
  end

  # @behavior RS-030
  def test_the_raiders_march_on_the_base
    add_child_autofree(@battle)
    raider = @battle.raiders.first
    start = raider.position.x

    wait_physics_frames 2

    assert_operator raider.position.x, :>, start
  end

  # @behavior RS-017
  def test_the_battle_ends_cleared_once_its_last_raider_is_destroyed
    add_child_autofree(@battle)

    assert wait_for_signal(Godot::Signal.new(@battle, :finished), 3), "the battle did not end"

    assert_empty @battle.raiders
    refute @battle.breached
  end
end
