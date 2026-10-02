# Times reaching each library file by name, giving a node each node script,
# which Godot answers from the script's ancestry, and walking the tree the
# Walk node walked in GDScript; prints each measure as
# `bench <name> <microseconds>` and quits.
class Bench < Godot::Node
  # How many times the tree is walked, as walk.gd walks it.
  PASSES = 20

  def _ready
    measure("run_by_name") do
      Manifest::NAMES.each { |path| path.inject(Object) { |scope, name| scope.const_get(name) } }
    end
    measure("attach") do
      Manifest::SCRIPTS.each do |script|
        node = Godot::Node.new
        node.set_script(Godot::ResourceLoader.load(script))
        node.free
      end
    end
    tree = get_node("Walk/Tree")
    measure("walk_ruby") { PASSES.times { walk(tree) } }
    get_tree.quit
  end

  def walk(node)
    node.get_children.inject(1) { |count, child| count + walk(child) }
  end

  def measure(name)
    started = Godot::Time.get_ticks_usec
    yield
    puts "bench #{name} #{Godot::Time.get_ticks_usec - started}"
  end
end
