# A turret firing on the enemy nearest to it among its siblings within its
# range, one shot each reload.
class Turret < Godot::Node2D
  export :mode, :nearest
  export :range, 300.0

  # Seconds between two shots.
  RELOAD_TIME = 0.1

  def _physics_process(delta)
    @cooldown = (@cooldown || 0.0) - delta
    return if @cooldown > 0.0

    target = find_target
    return unless target

    target.hit
    @cooldown = RELOAD_TIME
  end

  private

  def find_target
    candidates = get_parent.enemies.select { |enemy| measure_distance(enemy) <= range }
    candidates.inject { |near, enemy| measure_distance(enemy) < measure_distance(near) ? enemy : near }
  end

  def measure_distance(enemy)
    position.distance_to(enemy.position)
  end
end
