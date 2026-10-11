class PanicTest < Minitest::Test
  # @behavior RS-071
  def test_a_failure_inside_the_extension_as_godot_reads_a_node_answers_nothing
    node = autofree(Godot::Node.new)
    node.set_script(Godot::ResourceLoader.load("res://test/unit/script/babbler.rb"))
    node.call(:greet)

    assert_nil node.get(:mood)
  end
end
