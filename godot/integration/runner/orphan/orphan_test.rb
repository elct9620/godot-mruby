# A test that leaves a node outside the tree, which the run warns of as an
# orphan; the node is freed once the warning is made, so nothing leaks.
class OrphanTest < Minitest::Test
  def test_leaves_an_orphan
    @orphan = Godot::Node.new
  end

  def after_teardown
    super
    @orphan.free
  end
end
