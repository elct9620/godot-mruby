class RerunTest < Minitest::Test
  DIRECTORY = "res://test/unit/reload".freeze

  COUNTER = <<~RUBY.freeze
    module Unit
      module Reload
        class Counter < Godot::Node
          def bump
            @count = (@count || 0) + 1
          end

          def answer
            "after"
          end

          def added
            "added"
          end
        end
      end
    end
  RUBY

  UNFINISHED = <<~RUBY.freeze
    module Unit
      module Reload
        class Counter < Godot::Node
          LIMIT = 3
          raise "Counter is not finished"
        end
      end
    end
  RUBY

  BROKEN = <<~RUBY.freeze
    module Unit
      module Reload
        class Broken < Godot::Node
          def answer
            "mended"
          end
        end
      end
    end
  RUBY

  FAILING = <<~RUBY.freeze
    module Unit
      module Reload
        class Failing < Godot::Node
          def answer
            "mended"
          end
        end
      end
    end
  RUBY

  EXPORTING = <<~RUBY.freeze
    module Unit
      module Reload
        class Exporting < Godot::Node
          def speed?
            instance_variable_defined?(:@speed)
          end
        end
      end
    end
  RUBY

  # @behavior RF-002
  def test_a_reloaded_node_scripts_object_answers_a_method_only_its_changed_source_defines
    node = with_node(:counter)
    node.call(:answer)

    reloading(:counter, COUNTER) { assert_equal "added", node.call(:added) }
  end

  # @behavior RF-003
  def test_a_reloaded_node_scripts_object_keeps_its_state
    node = with_node(:counter)
    node.call(:bump)

    reloading(:counter, COUNTER) { assert_equal 2, node.call(:bump) }
  end

  # @behavior RF-004
  def test_a_method_the_changed_source_no_longer_defines_stays_until_the_game_restarts
    node = with_node(:counter)
    node.call(:answer)

    reloading(:counter, COUNTER) { assert_equal "dropped", node.call(:dropped) }
  end

  # @behavior RF-005
  def test_a_changed_source_that_raises_leaves_the_class_as_it_was
    node = with_node(:counter)
    node.call(:answer)

    reloading(:counter, UNFINISHED) { assert_equal "before", node.call(:answer) }
  end

  # @behavior RF-009
  def test_a_changed_source_that_raises_keeps_the_constants_it_assigned_again
    with_node(:counter).call(:answer)

    reloading(:counter, UNFINISHED) { assert Unit::Reload::Counter.const_defined?(:LIMIT, false) }
  end

  # @behavior RF-006
  def test_a_node_script_whose_file_raised_makes_objects_again_once_its_changed_source_runs
    with_node(:broken).call(:answer)

    reloading(:broken, BROKEN) { assert_equal "mended", with_node(:broken).call(:answer) }
  end

  # @behavior RF-007
  def test_a_node_whose_object_failed_stays_without_one_after_its_file_reloads
    node = with_node(:failing)
    node.call(:answer)

    reloading(:failing, FAILING) { assert_nil node.call(:answer) }
  end

  # @behavior RF-008
  def test_a_property_the_changed_source_no_longer_exports_is_not_given_to_a_new_object
    with_node(:exporting).call(:speed?)

    reloading(:exporting, EXPORTING) { refute with_node(:exporting).call(:speed?) }
  end

  private

  def script(name)
    Godot::ResourceLoader.load("#{DIRECTORY}/#{name}.rb")
  end

  def with_node(name)
    node = autofree(Godot::Node.new)
    node.set_script(script(name))
    node
  end

  # Runs the block once the script of that name has reloaded holding
  # `source`, then reloads it with what it held.
  def reloading(name, source)
    held = script(name).source_code
    reload(name, source)
    yield
  ensure
    reload(name, held)
  end

  def reload(name, source)
    script(name).source_code = source
    script(name).reload(true)
    wait_process_frames 1
  end
end
