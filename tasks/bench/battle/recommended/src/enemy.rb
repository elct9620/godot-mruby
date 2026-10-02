# The enemy of src/enemy.rb written as the call-cost analysis recommends: its
# position kept in Ruby and only written back, and moved by the battle rather
# than by a callback of its own.
class Enemy < Godot::Node2D
  signal :arrived

  export :speed, 200.0
  export :health, 3

  attr_accessor :goal
  attr_reader :dead

  # A red square marking where the enemy is.
  def _ready
    mark = Godot::ColorRect.new
    mark.color = Godot::Color.new(0.9, 0.2, 0.2)
    mark.position = Godot::Vector2.new(-8, -8)
    mark.size = Godot::Vector2.new(16, 16)
    add_child(mark)
    at = position
    @x = at.x
    @y = at.y
  end

  # Moves by one frame; whether the enemy is still short of its goal.
  def advance(delta)
    @x += speed * delta
    self.position = Godot::Vector2.new(@x, @y)
    @x < goal
  end

  def hit
    self.health -= 1
    return if health > 0

    @dead = true
    queue_free
  end
end
