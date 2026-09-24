# One wave of enemies marching on the base while the turret fires on them:
# the battle ends cleared once the last enemy is destroyed, or breached as
# soon as one reaches the base.
class Battle < Godot::Node2D
  signal :finished

  export :wave, 4
  export :spacing, 40.0

  attr_reader :breached

  def _ready
    goal = get_node("Base").position.x
    wave.times do |index|
      enemy = Enemy.new
      enemy.position = Godot::Vector2.new(-spacing * index, 0)
      enemy.goal = goal
      enemy.connect(:arrived, method(:breach))
      add_child(enemy)
    end
  end

  # The scene's own children arrive before it enters the tree, and a freed
  # battle loses its enemies after leaving it; neither ends a battle.
  def _notification(what)
    finish if what == NOTIFICATION_CHILD_ORDER_CHANGED && is_inside_tree && enemies.empty?
  end

  def enemies
    get_children.select { |child| child.is_a?(Enemy) && !child.is_queued_for_deletion }
  end

  private

  def breach
    @breached = true
    finish
  end

  def finish
    return if @finished

    @finished = true
    emit_signal(:finished)
  end
end
