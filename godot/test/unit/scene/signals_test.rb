class SignalsTest < Minitest::Test
  def setup
    @node = autofree(Godot::Node.new)
  end

  # Checks, as teardown begins, that the node watched has the connections it
  # had before.
  def teardown
    assert_equal @connections, connection_count if @connections
    super
  end

  # @behavior RW-016
  def test_a_signal_a_watched_object_emitted_passes_assert_signal_emitted
    watch_signals(@node)

    @node.notify_property_list_changed

    assert_signal_emitted @node, :property_list_changed
    assert_signal_emitted Godot::Signal.new(@node, :property_list_changed)
  end

  # @behavior RW-017
  def test_a_signal_a_watched_object_did_not_emit_fails_assert_signal_emitted
    watch_signals(@node)

    assert_raises(Minitest::Assertion) { assert_signal_emitted @node, :property_list_changed }
  end

  # @behavior RW-018
  def test_an_object_nobody_watched_fails_assert_signal_emitted
    @node.notify_property_list_changed

    assert_raises(Minitest::Assertion) { assert_signal_emitted @node, :property_list_changed }
  end

  # @behavior RW-019
  def test_a_signal_a_watched_object_does_not_have_fails_assert_signal_emitted
    watch_signals(@node)

    assert_raises(Minitest::Assertion) { assert_signal_emitted @node, :absent }
  end

  # @behavior RW-020
  def test_a_watched_object_has_the_connections_it_had_once_the_teardown_begins
    @connections = connection_count

    watch_signals(@node)

    assert_operator connection_count, :>, @connections
  end

  private

  def connection_count
    @node.get_signal_list.inject(0) { |count, declared| count + @node.get_signal_connection_list(declared["name"]).size }
  end
end
