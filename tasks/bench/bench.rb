# Times reaching each library file by name, then giving a node each node
# script, which Godot answers from the script's ancestry; prints each measure
# as `bench <name> <microseconds>` and quits.
class Bench < Godot::Node
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
    get_tree.quit
  end

  def measure(name)
    started = Godot::Time.get_ticks_usec
    yield
    puts "bench #{name} #{Godot::Time.get_ticks_usec - started}"
  end
end
