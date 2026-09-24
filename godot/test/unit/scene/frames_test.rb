class FramesTest < Minitest::Test
  # @behavior RW-001
  def test_a_test_waiting_for_process_frames_continues_after_that_many_frames
    before = Godot::Engine.get_process_frames

    wait_process_frames 3

    assert_equal before + 3, Godot::Engine.get_process_frames
  end

  # @behavior RW-006
  def test_a_test_waiting_for_physics_frames_continues_inside_the_last_of_them
    before = Godot::Engine.get_physics_frames

    wait_physics_frames 3

    assert_equal before + 3, Godot::Engine.get_physics_frames
    assert Godot::Engine.is_in_physics_frame
  end

  # A test begins by the step each test's before_setup takes, called here
  # where the test stands, since which test runs after this one is the seed's.
  # @behavior RW-007
  def test_a_test_begins_where_a_process_frame_begins_after_one_that_waited_into_a_physics_frame
    wait_physics_frames 1

    begin_at_process_frame

    refute Godot::Engine.is_in_physics_frame
  end
end
