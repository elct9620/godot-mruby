# An enemy walking toward its goal, destroyed once it has taken as many hits
# as its health.
class Enemy < Godot::Node2D
  signal :arrived

  export :speed, 200.0
  export :health, 3

  attr_accessor :goal

  # A red square marking where the enemy is.
  def _ready
    mark = Godot::ColorRect.new
    mark.color = Godot::Color.new(0.9, 0.2, 0.2)
    mark.position = Godot::Vector2.new(-8, -8)
    mark.size = Godot::Vector2.new(16, 16)
    add_child(mark)
  end

  def _physics_process(delta)
    self.position = position + Godot::Vector2.new(speed * delta, 0)
    return if position.x < goal

    emit_signal(:arrived)
    queue_free
  end

  def hit
    self.health -= 1
    queue_free if health <= 0
  end
end
