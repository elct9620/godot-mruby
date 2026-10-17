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

  # Each node is a process group of its own, so Godot processes them on its
  # worker threads at once, and the realm lets one in at a time.
  # @behavior RE-018
  def test_nodes_in_sub_thread_process_groups_run_their_ruby_off_the_main_thread
    treadmills = Array.new(2) { Unit::Entry::Treadmill.new }
    treadmills.each do |treadmill|
      treadmill.process_thread_group = Godot::Node::PROCESS_THREAD_GROUP_SUB_THREAD
      add_child_autofree(treadmill)
    end

    wait_process_frames(10)

    assert_equal [true, true], treadmills.map { |treadmill| treadmill.steps >= 10 }
    refute_includes treadmills.flat_map(&:threads), Godot::OS.get_main_thread_id
  end
end
