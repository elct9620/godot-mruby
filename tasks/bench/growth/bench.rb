# Times reaching each library file by name, giving a node each node script,
# which Godot answers from the script's ancestry, walking the tree the Walk
# node walked in GDScript, and reading a node's name, which the engine hands
# Ruby as a symbol among every name the project's files gave the realm;
# prints each measure as `bench <name> <microseconds>` and quits.
class Bench < Godot::Node
  # How many times the tree is walked, as walker.gd walks it.
  PASSES = 20
  # How many times the node's name is read.
  NAME_READS = 10_000

  def _ready
    # The name is read once before the files run, so it is a symbol the
    # realm holds from the start, as a game's names are, and the files' names
    # come after it.
    named = Godot::Node.new
    named.set_name("growth_marker")
    named.get_name
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
    measure("name_get") { NAME_READS.times { named.get_name } }
    named.free
    get_tree.quit
  end

  def walk(node)
    node.get_children.inject(1) { |count, child| count + walk(child) }
  end

  def measure(name)
    start = Godot::Time.get_ticks_usec
    yield
    puts "bench #{name} #{Godot::Time.get_ticks_usec - start}"
  end
end
