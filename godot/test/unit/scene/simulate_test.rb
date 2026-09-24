class SimulateTest < Minitest::Test
  # @behavior RW-021
  def test_simulate_processes_a_node_and_its_children_as_many_times_as_asked
    parent = autofree(ticker)
    child = ticker
    parent.add_child(child)

    simulate(parent, 3, 0.5)

    [parent, child].each do |node|
      assert_equal [0.5, 0.5, 0.5], node.call(:processed)
      assert_equal [0.5, 0.5, 0.5], node.call(:physics_processed)
    end
  end

  # @behavior RW-022
  def test_simulate_checking_processing_leaves_a_node_that_is_not_processing
    node = autofree(ticker)

    simulate(node, 1, 0.5, true)

    assert_empty node.call(:processed)
    assert_empty node.call(:physics_processed)
  end

  private

  # A node outside the tree, so only simulate processes it.
  def ticker
    node = Godot::Node.new
    node.set_script(Godot::ResourceLoader.load("res://test/unit/scene/ticker.rb"))
    node
  end
end
