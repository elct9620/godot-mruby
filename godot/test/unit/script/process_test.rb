class ProcessTest < Minitest::Test
  # @behavior RS-012
  def test_process_is_given_the_frames_delta_as_a_float
    node = Godot::Node.new
    node.set_script(Godot::ResourceLoader.load("res://test/unit/script/ticking.rb"))
    add_child_autofree(node)

    wait_process_frames 1

    assert_instance_of Float, node.call(:delta)
  end
end
