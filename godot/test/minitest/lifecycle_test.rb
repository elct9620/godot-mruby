# Runs a test class built inside each test, so its hooks can be watched from
# outside; the runner never runs it, since it did not exist when the run
# began.
class LifecycleTest < Minitest::Test
  # @behavior RM-002
  def test_the_hooks_run_around_the_test_in_minitests_order
    steps = []
    recorder = Class.new(Minitest::Test) do
      %w[before_setup setup after_setup test_it before_teardown teardown after_teardown].each do |step|
        define_method(step) { steps << step }
      end
    end

    recorder.new("test_it").run

    assert_equal %w[before_setup setup after_setup test_it before_teardown teardown after_teardown], steps
  end

  # @behavior RM-003
  def test_a_teardown_step_that_raises_does_not_stop_the_ones_after_it
    steps = []
    recorder = Class.new(Minitest::Test) do
      define_method(:before_teardown) { raise ArgumentError, "teardown failed" }
      define_method(:teardown) { steps << "teardown" }
      define_method(:after_teardown) { steps << "after_teardown" }
      define_method(:test_it) {}
    end

    result = recorder.new("test_it").run

    assert_equal %w[teardown after_teardown], steps
    assert_equal "E", result.result_code
  end
end
