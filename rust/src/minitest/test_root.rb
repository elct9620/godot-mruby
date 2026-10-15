# A test's way into the scene: each test is given a test root in the tree
# under the scene the test runner runs, and what the test adds under it or
# gives to autofree is freed once its teardown has run, and what it gives to
# autoqfree is queued to be freed then, a frame passing before orphans are
# counted; a node the test left outside the tree is warned of.
module Minitest
  module TestRoot
    def before_setup
      super
      @orphans = orphan_count
      @autofree_objects = []
      @autoqfree_nodes = []
      @test_root = Godot::Node.new
      Godot::Engine.get_main_loop.current_scene.add_child(@test_root)
    end

    def add_child_autofree(node)
      test_root.add_child(node)
      autofree(node)
    end

    def autofree(object)
      @autofree_objects << object
      object
    end

    def autoqfree(node)
      @autoqfree_nodes << node
      node
    end

    def after_teardown
      @autofree_objects.each { |object| free_unless_freed(object) }
      @autoqfree_nodes.each { |node| free_unless_freed(node, :queue_free) }
      free_unless_freed(@test_root)
      wait_process_frames(1) unless @autoqfree_nodes.empty?
      warn_of_orphans
      super
    end

    private

    # Private, as a public method named test_ would be run as a test.
    def test_root
      @test_root
    end

    # A node outside the tree that nothing frees is an orphan; as GUT does,
    # the test that left one is named in a warning rather than failed.
    def warn_of_orphans
      orphans = orphan_count - @orphans
      return unless orphans > 0

      Godot.push_warning("#{self.class}##{name} leaves #{orphans} orphan #{orphans == 1 ? "node" : "nodes"}")
    end

    def orphan_count
      Godot::Performance.get_monitor(Godot::Performance::OBJECT_ORPHAN_NODE_COUNT).to_i
    end

    # Calling a freed engine object raises Godot::CallError, so a node the
    # test freed itself is left alone.
    def free_unless_freed(object, free_method = :free)
      object.__send__(free_method)
    rescue Godot::CallError
      nil
    end
  end

  class Test
    include TestRoot
  end
end
