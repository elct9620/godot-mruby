# A test that fails only after a frame has passed: the run has to go on past
# the frame it started in, report the failure, and fail.
class WaitedFailureTest < Minitest::Test
  def test_fails_after_a_frame
    wait_process_frames 1
    assert_equal 1, 2
  end
end
