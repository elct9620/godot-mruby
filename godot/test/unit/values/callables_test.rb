class CallablesTest < Minitest::Test
  # A Ruby object of a class of its own, which no engine type fits.
  class Satchel
    def double(value)
      value * 2
    end
  end

  def setup
    @node = autofree(Godot::Node.new)
  end

  # @behavior RV-017
  def test_a_proc_or_method_crosses_as_a_callable_running_it
    doubling = proc { |value| value * 2 }
    satchel = Satchel.new

    assert_equal 42, round_trip(doubling).call(21)
    assert_equal 42, round_trip(satchel.method(:double)).call(21)
  end

  # @behavior RV-018
  def test_a_proc_connected_to_a_signal_runs_when_the_signal_is_emitted
    changed = 0
    @node.connect(:property_list_changed, proc { changed += 1 })

    @node.notify_property_list_changed

    assert_equal 1, changed
  end

  # @behavior RV-019
  def test_any_other_ruby_object_crosses_as_the_same_object
    satchel = Satchel.new
    @node.set_meta(:satchel, satchel)

    assert_same satchel, @node.get_meta(:satchel)
    assert_same satchel, round_trip([satchel]).first
    assert Godot::ClassDB.class_exists("RubyObject")
  end

  # @behavior RV-020
  def test_an_engine_callable_crosses_as_a_value_ruby_calls
    callable = Godot::Callable.new(@node, :get_child_count)

    assert_instance_of Godot::Callable, callable
    assert_equal 0, callable.call
  end

  private

  def round_trip(value)
    @node.get_meta(:absent, value)
  end
end
