class TestRootTest < Minitest::Test
  # Checks, once the test root has freed what it holds, that each node a test
  # expects to have been freed with it has.
  def after_teardown
    super
    (@expect_freed || []).each do |node|
      assert_raises(Godot::CallError) { node.get_name }
    end
  end

  # @behavior RW-002
  def test_a_node_added_with_add_child_autofree_goes_under_the_test_root
    node = add_child_autofree(Godot::Node.new)

    assert_equal test_root, node.get_parent
  end

  # @behavior RW-003
  def test_a_node_given_to_autofree_is_freed_once_the_teardown_has_run
    @expect_freed = [autofree(Godot::Node.new)]
  end

  # @behavior RW-004
  def test_a_node_under_the_test_root_is_freed_with_it_once_the_teardown_has_run
    node = Godot::Node.new
    test_root.add_child(node)
    @expect_freed = [node]
  end

  # @behavior RW-005
  def test_a_node_the_test_freed_itself_is_left_to_it
    autofree(Godot::Node.new).free
  end
end
