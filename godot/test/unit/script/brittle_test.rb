class BrittleTest < Minitest::Test
  # @behavior RS-066
  def test_a_node_whose_object_failed_to_build_keeps_the_values_godot_writes_to_it
    node = autofree(Godot::Node.new)
    node.set_script(Godot::ResourceLoader.load("res://test/unit/script/brittle.rb"))

    node.set(:armor, 5)

    assert_equal 5, node.get(:armor)
  end
end
