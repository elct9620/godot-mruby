# Times each kind of call a game makes, many times over, and prints the
# nanoseconds of one: Ruby's own method, the engine's methods and
# properties, an engine singleton's method, a utility function, and the
# engine's Vector2 and Color. The loop's own time is printed too, for the
# bench to take away.
class Ops < Godot::Node2D
  N = 100_000

  def _ready
    v = Godot::Vector2.new(1, 2)
    w = Godot::Vector2.new(3, 4)
    from = v.x
    to = w.x
    measure("loop") { N.times { nil } }
    measure("ruby_call") { N.times { noop } }
    measure("engine_call") { N.times { get_child_count } }
    measure("position_get") { N.times { position } }
    measure("position_set") { N.times { self.position = v } }
    measure("singleton_call") { N.times { Godot::Engine.get_process_frames } }
    measure("utility_call") { N.times { Godot.lerp(from, to, 0.5) } }
    measure("vector_new") { N.times { Godot::Vector2.new(1, 2) } }
    measure("color_new") { N.times { Godot::Color.new(1, 0, 0) } }
    measure("vector_add") { N.times { v + w } }
    measure("vector_x") { N.times { v.x } }
    get_tree.quit
  end

  def noop; end

  def measure(name)
    started = Godot::Time.get_ticks_usec
    yield
    puts "op #{name} #{(Godot::Time.get_ticks_usec - started) * 1000.0 / N}"
  end
end
