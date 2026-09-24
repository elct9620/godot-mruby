class FramesTest < Minitest::Test
  # @behavior RW-001
  def test_a_test_waiting_for_process_frames_continues_after_that_many_frames
    before = Godot::Engine.get_process_frames

    wait_process_frames 3

    assert_equal before + 3, Godot::Engine.get_process_frames
  end
end
