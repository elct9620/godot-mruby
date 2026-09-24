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

  # @behavior RW-008
  def test_a_test_waiting_for_seconds_continues_once_that_much_physics_time_has_passed
    delta = test_root.get_physics_process_delta_time
    before = Godot::Engine.get_physics_frames

    wait_seconds 0.1

    assert_in_delta 0.1, (Godot::Engine.get_physics_frames - before) * delta, delta
  end

  # @behavior RW-009
  def test_wait_until_answers_true_once_its_block_answers_true
    calls = 0

    answer = wait_until(1) { (calls += 1) == 3 }

    assert_equal true, answer
    assert_equal 3, calls
  end

  # @behavior RW-010
  def test_wait_until_answers_false_once_max_time_passes_with_no_answer_of_true
    delta = test_root.get_physics_process_delta_time
    before = Godot::Engine.get_physics_frames

    answer = wait_until(0.1) { 1 }

    assert_equal false, answer
    assert_in_delta 0.1, (Godot::Engine.get_physics_frames - before) * delta, delta
  end

  # @behavior RW-011
  def test_wait_until_given_a_time_between_calls_calls_its_block_no_more_often
    calls = 0

    wait_until(0.1, 0.05) { (calls += 1) && false }

    assert_operator calls, :<=, 2
  end

  # @behavior RW-012
  def test_wait_until_without_a_block_raises_argument_error
    assert_raises(ArgumentError) { wait_until(0.1) }
  end
end
