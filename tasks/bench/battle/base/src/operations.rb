# Times each kind of call a game makes, many times over, and prints the
# nanoseconds of one: Ruby's own method, the engine's methods and
# properties, an engine singleton's method, a utility function, and the
# engine's Vector2 and Color. The loop's own time is printed too, for the
# bench to take away.
class Operations < Godot::Node2D
  REPEATS = 100_000

  def _ready
    v = Godot::Vector2.new(1, 2)
    w = Godot::Vector2.new(3, 4)
    from = v.x
    to = w.x
    measure("loop") { REPEATS.times { nil } }
    measure("ruby_call") { REPEATS.times { noop } }
    measure("engine_call") { REPEATS.times { get_child_count } }
    measure("position_get") { REPEATS.times { position } }
    measure("position_set") { REPEATS.times { self.position = v } }
    measure("member_set") { REPEATS.times { position.y = 5.0 } }
    measure("singleton_call") { REPEATS.times { Godot::Engine.get_process_frames } }
    measure("utility_call") { REPEATS.times { Godot.lerp(from, to, 0.5) } }
    measure("vector_new") { REPEATS.times { Godot::Vector2.new(1, 2) } }
    measure("color_new") { REPEATS.times { Godot::Color.new(1, 0, 0) } }
    measure("vector_add") { REPEATS.times { v + w } }
    measure("vector_x") { REPEATS.times { v.x } }
    get_tree.quit
  end

  def noop; end

  def measure(name)
    start = Godot::Time.get_ticks_usec
    yield
    puts "op #{name} #{(Godot::Time.get_ticks_usec - start) * 1000.0 / REPEATS}"
  end
end
