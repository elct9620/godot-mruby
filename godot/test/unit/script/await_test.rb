class AwaitTest < Minitest::Test
  # @behavior RS-094
  def test_a_method_godot_calls_returns_as_it_awaits
    sentinel, source = make_pair

    answered = sentinel.call(:watch, source)

    assert_kind_of Godot::Signal, answered
    assert_equal [[:started], 1], [sentinel.log, source.get_signal_connection_list(:relayed).size]
  end

  # @behavior RS-095
  def test_an_awaiting_method_resumes_once_its_signal_is_emitted
    sentinel, source = make_pair
    sentinel.call(:watch, source)

    source.emit_signal(:relayed)

    assert_equal [:started, nil], sentinel.log
  end

  # @behavior RS-096
  def test_await_answers_what_the_signal_was_emitted_with
    answers = [[], [7], [7, 8]].map do |values|
      sentinel, source = make_pair
      sentinel.call(:watch, source)
      source.emit_signal(:relayed, *values)
      sentinel.log.last
    end

    assert_equal [nil, 7, [7, 8]], answers
  end

  # @behavior RS-097
  def test_a_block_godot_calls_back_may_await
    sentinel, source = make_pair
    sentinel.arm(source)

    source.emit_signal(:pinged)
    source.emit_signal(:relayed, 5)

    assert_equal [5], sentinel.log
  end

  # @behavior RS-098
  def test_await_outside_what_godot_calls_raises
    sentinel, source = make_pair

    assert_raises(FiberError) { sentinel.watch(source) }
  end

  # @behavior RS-099
  def test_a_method_godot_calls_awaits_through_the_methods_it_calls
    sentinel, source = make_pair
    sentinel.call(:relay, source)

    source.emit_signal(:relayed, 1)
    source.emit_signal(:relayed, 2)

    assert_equal [1, 2], sentinel.log
  end

  # @behavior RS-100
  def test_gdscript_awaiting_a_ruby_method_waits_until_it_ends
    sentinel, source = make_pair
    waiter = gdscript_waiter
    waiter.call(:run, sentinel, source)
    before = waiter.get(:result)

    source.emit_signal(:relayed, 7)

    assert_nil before
    assert_equal [:echoed, 7], waiter.get(:result).to_a
  end

  # @behavior RS-101
  def test_a_method_that_could_await_but_does_not_answers_its_value
    sentinel, source = make_pair

    assert_equal :early, sentinel.call(:maybe, source, false)
  end

  # @behavior RS-102
  def test_await_answers_a_value_that_is_no_signal_at_once
    sentinel, = make_pair

    assert_equal 3, sentinel.call(:await_plain)
    assert_equal 5, sentinel.await(5)
  end

  # @behavior RS-103
  def test_godot_coroutine_run_goes_on_without_waiting
    sentinel, source = make_pair
    answered = sentinel.call(:spawn, source)
    went_on = sentinel.log.dup
    sentinel.call(:join)

    source.emit_signal(:relayed, 4)

    assert_kind_of Godot::Signal, answered
    assert_equal [:went_on], went_on
    assert_equal [:went_on, 4, :block_done], sentinel.log
  end

  # @behavior RS-104
  def test_await_under_a_method_mruby_runs_in_c_raises_and_leaves_no_connection
    sentinel, source = make_pair
    sentinel.call(:sort_awaiting, source)
    connected = source.get_signal_connection_list(:relayed).size

    source.emit_signal(:relayed, 6)

    assert_equal 1, connected
    assert_equal [:refused, 6], sentinel.log
  end

  # @behavior RS-105
  def test_a_coroutine_whose_signal_goes_with_its_object_is_let_go_of
    sentinel = autofree(Unit::Script::Sentinel.new)
    source = Godot::Node.new
    source.add_user_signal("relayed")
    state = spawn_state_id(sentinel, source)

    source.free
    wait_process_frames(1)
    GC.start
    wait_process_frames(1)

    refute instance_id_valid?(state)
  end

  private

  # The state `spawn` answers its block's end through, made in a fiber that
  # ends, whose registers the collector no longer marks.
  def spawn_state_id(sentinel, source)
    Fiber.new { sentinel.call(:spawn, source).get_object_id }.resume
  end

  # Asked through an expression, which takes no reference to the object.
  def instance_id_valid?(id)
    expression = Godot::Expression.new
    expression.parse("is_instance_id_valid(id)", ["id"])
    expression.execute([id])
  end

  # A GDScript object whose `run` awaits a node's `echo`, as GDScript code
  # awaiting a Ruby method does, keeping what it answered.
  def gdscript_waiter
    script = Godot::GDScript.new
    script.source_code = "extends RefCounted\nvar result\nfunc run(node, source):\n\tresult = await node.echo(source)\n"
    script.reload
    script.new
  end

  def make_pair
    source = autofree(Godot::Node.new)
    source.add_user_signal("relayed")
    source.add_user_signal("pinged")
    [autofree(Unit::Script::Sentinel.new), source]
  end
end
