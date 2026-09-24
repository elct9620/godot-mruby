class EntryTest < Minitest::Test
  # How deep one thread nests its entries before the next one fails.
  DEEPEST_ENTRY = 24

  # A thread the engine starts gets a stack of the platform's choosing, so
  # nesting as deep as the realm allows has to fit there too.
  # @behavior RE-006
  def test_entries_nested_too_deep_on_an_engine_thread_raise_system_stack_error
    node = autofree(Godot::Node.new)
    depths = []
    node.add_user_signal("nested")
    node.connect(:nested, proc { |depth|
      depths << depth
      node.emit_signal(:nested, depth + 1)
    })
    thread = Godot::Thread.new

    thread.start(proc { node.emit_signal(:nested, 1) })
    finished = wait_until(5) { !thread.is_alive }
    # Joining a thread still running Ruby would wait for good, since the main
    # thread would then hold the realm it waits on.
    thread.wait_to_finish if finished

    assert finished
    assert_equal (1...DEEPEST_ENTRY).to_a, depths
  end
end
