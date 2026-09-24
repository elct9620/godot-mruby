# A turret firing on the raider nearest to it among its siblings within its
# range, one shot each reload.
class Turret < Godot::Node2D
  export :mode, :nearest
  export :range, 300.0

  # Seconds between two shots.
  RELOAD = 0.1

  def _physics_process(delta)
    @cooldown = (@cooldown || 0.0) - delta
    return if @cooldown > 0.0

    target = nearest
    return unless target

    target.hit
    @cooldown = RELOAD
  end

  private

  def nearest
    in_range = get_parent.raiders.select { |raider| distance(raider) <= range }
    in_range.inject { |near, raider| distance(raider) < distance(near) ? raider : near }
  end

  def distance(raider)
    position.distance_to(raider.position)
  end
end
