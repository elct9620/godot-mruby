# A test that queues its node outside the tree to be freed with autoqfree,
# which the run frees before counting orphans, so it warns of none.
class AutoqfreeTest < Minitest::Test
  def test_queues_its_node
    autoqfree(Godot::Node.new)
  end
end
