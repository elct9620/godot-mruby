# One wave of raiders marching on the base while the turret fires on them:
# the battle ends cleared once the last raider is destroyed, or breached as
# soon as one reaches the base.
class Battle < Godot::Node2D
  signal :finished

  export :wave, 4
  export :spacing, 40.0

  attr_reader :breached

  def _ready
    goal = get_node("Base").position.x
    wave.times do |index|
      raider = Raider.new
      raider.position = Godot::Vector2.new(-spacing * index, 0)
      raider.goal = goal
      raider.connect(:arrived, method(:breach))
      add_child(raider)
    end
  end

  # The scene's own children arrive before it enters the tree, and a freed
  # battle loses its raiders after leaving it; neither ends a battle.
  def _notification(what)
    finish if what == NOTIFICATION_CHILD_ORDER_CHANGED && is_inside_tree && raiders.empty?
  end

  def raiders
    get_children.select { |child| child.is_a?(Raider) && !child.is_queued_for_deletion }
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
