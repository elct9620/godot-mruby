# The battle of src/battle.rb written as the call-cost analysis recommends:
# it keeps its enemies in Ruby and moves them all in one callback, after the
# turret has fired, as their own callbacks did.
class Battle < Godot::Node2D
  signal :finished

  export :wave, 4
  export :spacing, 40.0

  attr_reader :breached

  def _ready
    @roster = []
    goal = get_node("Base").position.x
    wave.times do |index|
      enemy = Enemy.new
      enemy.position = Godot::Vector2.new(-spacing * index, 0)
      enemy.goal = goal
      @roster << enemy
      add_child(enemy)
    end
    self.process_physics_priority = 1
  end

  def _physics_process(delta)
    @roster.dup.each do |enemy|
      next @roster.delete(enemy) if enemy.dead
      next if enemy.advance(delta)

      @roster.delete(enemy)
      enemy.queue_free
      breach
    end
    nil
  end

  # The scene's own children arrive before it enters the tree, and a freed
  # battle loses its enemies after leaving it; neither ends a battle.
  def _notification(what)
    finish if what == NOTIFICATION_CHILD_ORDER_CHANGED && is_inside_tree && enemies.empty?
  end

  def enemies
    @roster.reject(&:dead)
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
